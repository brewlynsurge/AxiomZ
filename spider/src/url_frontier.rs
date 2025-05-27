use futures::future::join_all;
use rayon::prelude::*;
use std::cmp::{Ordering, min};
use std::collections::{BinaryHeap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;

use rand::distr::weighted::WeightedIndex;
use rand::prelude::*;
use url::Url;

use crate::utils;
use database::database::SurfXDatabase;

#[derive(Debug, Clone)]
struct FrontierUrl {
    url: String,
    priority: f64,
}

impl FrontierUrl {
    pub fn new(url: &str, priority: f64) -> Option<Self> {
        if Url::parse(url).is_ok() {
            Some(Self {
                url: url.to_string(),
                priority,
            })
        } else {
            None
        }
    }
}

impl PartialEq for FrontierUrl {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for FrontierUrl {}

impl PartialOrd for FrontierUrl {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FrontierUrl {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .partial_cmp(&self.priority)
            .unwrap_or(Ordering::Equal)
    }
}

pub struct UrlFrontier {
    urls: BinaryHeap<FrontierUrl>,
    previous_url: Option<String>,
    start_url: String,
}
impl UrlFrontier {
    pub fn new(start_url: &str) -> Self {
        Self {
            urls: BinaryHeap::new(),
            previous_url: None,
            start_url: start_url.to_string(),
        }
    }

    pub async fn get_init_url(&mut self, database: Arc<Mutex<SurfXDatabase>>) -> String {
        if self.urls.is_empty() {
            return self.start_url.clone();
        } else {
            return self.get_url(database).await.unwrap();
        }
    }

    pub async fn get_url(&mut self, database: Arc<Mutex<SurfXDatabase>>) -> Option<String> {
        // Damping factor (default: 0.85)
        // 85% chance to choose a url from the frontier
        // 15% chance to choose a random url from the list
        let new_url =
            if Self::choose_random_action(&[0.85, 0.15]).unwrap() == 0 && !self.urls.is_empty() {
                // 75% chance to avoid same domain consecutively
                if self.previous_url.is_some()
                    && Self::choose_random_action(&[0.75, 0.25]).unwrap() == 0
                {
                    let previous_url = self.previous_url.as_ref()?;
                    let previous_url_domain = utils::get_url_base(previous_url)?.to_string();

                    let new_url = self.pop_url(database.clone()).await?;
                    let new_url_domain = utils::get_url_base(&new_url)?.to_string();

                    if previous_url_domain == new_url_domain {
                        self.pop_url(database.clone()).await?
                    } else {
                        new_url
                    }
                } else {
                    self.pop_url(database.clone()).await?
                }
            } else {
                let mut rng = rand::rng();
                utils::URLS.choose(&mut rng)?.to_string()
            };

        self.previous_url = Some(new_url.clone());
        return Some(new_url);
    }

    fn choose_random_action(weights: &[f64]) -> Result<usize, std::io::Error> {
        if weights.is_empty() || weights.iter().all(|&w| w <= 0.0) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Weights must be non-empty and contain at least one positive value",
            ));
        }

        let dist = WeightedIndex::new(weights).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("Invalid weights: {}", e))
        })?;

        let mut rng = rand::rng();
        Ok(dist.sample(&mut rng))
    }

    pub async fn extend(&mut self, urls: HashSet<String>, database: Arc<Mutex<SurfXDatabase>>) {
        // Process URLs in parallel
        let frontier_urls: Vec<_> = urls
            .into_par_iter()
            .filter_map(|url| {
                let priority = Self::calculate_priority(&url);
                FrontierUrl::new(&url, priority).map(|f| (url, priority, f))
            })
            .collect();

        // Asynchronous insertion with scoped locking
        let insert_futures = {
            let db = database.clone();
            frontier_urls.iter().map(move |(url, priority, _)| {
                let db = db.clone();
                let url = url.clone();
                let priority = *priority;
                async move {
                    let db = db.lock().await;
                    _ = db.insert_to_url_frontier(&url, priority).await;
                }
            })
        };
        join_all(insert_futures).await;

        // Add to in-memory frontier
        self.urls.extend(
            frontier_urls
                .into_iter()
                .map(|(_, _, frontier_url)| frontier_url),
        );
    }

    fn calculate_priority(url: &str) -> f64 {
        let mut priority = 0.5;
        let url = match Url::parse(url) {
            Ok(url) => url,
            Err(_) => return priority, // Return default if URL is invalid
        };
    
        // Domain-based rules
        let domain = url.domain().unwrap_or("").to_lowercase();
        
        // Rule 1: Boost for specific news domains
        let news_domains = vec!["nytimes.com", "bbc.com", "cnn.com"];
        if news_domains.iter().any(|&news_domain| domain.ends_with(news_domain)) {
            priority += 0.15;
        }
    
        // Rule 2: Boost for .gov or .edu domains
        if domain.ends_with(".gov") || domain.ends_with(".edu") {
            priority += 0.1;
        }
    
        // Rule 3: Boost for keywords in domain name
        if domain.contains("news") || domain.contains("blog") || domain.contains("article") {
            priority += 0.1;
        }
    
        // Rule 4: Penalize for excessive subdomains
        let subdomain_count = domain.split('.').count().saturating_sub(2);
        if subdomain_count > 2 {
            priority -= 0.1;
        }
    
        // Path-based rules
        let path = url.path().to_lowercase();
    
        // Rule 5: Adjust priority based on URL depth (capped at 3 segments)
        let path_depth = url.path_segments().map(|s| s.count()).unwrap_or(0);
        priority -= 0.15 * min(path_depth, 3) as f64;
    
        // Rule 6: Boost for positive path keywords
        let positive_keywords = vec![
            "news", "article", "blog", "wiki", "faq", "guide", "tutorial",
            "research", "study", "report", "analysis", "review", "press-release",
            "whitepaper", "case-study", "ebook", "infographic", "video", "podcast", "interview"
        ];
        if positive_keywords.iter().any(|&keyword| path.contains(keyword)) {
            priority += 0.2;
        }
    
        // Rule 7: Penalize for negative path keywords
        let negative_keywords = vec![
            "login", "signup", "cart", "checkout", "account", "profile", "settings",
            "help", "contact", "about", "privacy", "terms", "advertisement", "promo", "deal"
        ];
        if negative_keywords.iter().any(|&keyword| path.contains(keyword)) {
            priority -= 0.2;
        }
    
        // Rule 8: Adjust based on file extensions
        if path.ends_with(".html") || path.ends_with(".htm") {
            priority += 0.17;
        } else if path.ends_with(".pdf") || path.ends_with(".docx") {
            priority -= 0.3;
        }
    
        // Query-based rules
        // Rule 9: Penalize for excessive query parameters
        let query_params = url.query_pairs().count();
        if query_params > 3 {
            priority -= 0.1 * (query_params as f64 - 3.0);
        }
    
        // Rule 10: Penalize for tracking parameters
        let query = url.query().unwrap_or("");
        if query.contains("utm_source") || query.contains("utm_medium") || query.contains("utm_campaign") {
            priority -= 0.1;
        }
    
        // General URL properties
        // Rule 11: Adjust based on URL length
        let url_length = url.as_str().len();
        if url_length < 50 { priority += 0.05; }
        else if url_length > 100 { priority -= 0.05; }
    
        // Clamp priority between 0.0 and 1.0
        priority.clamp(0.0, 1.0)
    }

    async fn pop_url(&mut self, database: Arc<Mutex<SurfXDatabase>>) -> Option<String> {
        let url: String = self.urls.pop()?.url.clone();
        {
            let db = database.lock().await;
            _ = db.remove_from_url_frontier(&url).await;
        }

        Some(url)
    }

    pub async fn load_state(&mut self, database: Arc<Mutex<SurfXDatabase>>) {
        let db = database.lock().await;
        match db.get_all_urls_from_url_frontier().await {
            Ok(urls) => {
                self.urls.extend(
                    urls.into_iter()
                        .map(|(url, priority)| FrontierUrl::new(&url, priority).unwrap()),
                );
            }
            Err(_) => {}
        }
    }
}
