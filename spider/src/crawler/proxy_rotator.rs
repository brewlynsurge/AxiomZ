use std::sync::Arc;
use tokio::sync::Mutex;
use serde::Deserialize;
use rand::prelude::*;
use rand_distr::weighted::WeightedAliasIndex;


// ----------------- PROXY ENTRY ----------------------
pub struct AxiomZProxy {
    pub ip: String,
    pub port: u16,
    
    pub score: f64,
    pub cooldown: Option<tokio::time::Instant>
}

impl AxiomZProxy {
    pub fn is_available(&self) -> bool {
        match self.cooldown {
            Some(t) => {return tokio::time::Instant::now() >= t}
            None => {return true}
        }
    }

    pub fn set_cooldown(&mut self, duration: tokio::time::Duration) {
        self.cooldown = Some(tokio::time::Instant::now() + duration);
    }
}

// ----------------- PROXY ROTATOR ----------------------
pub struct ProxyRotator {
    proxies: Arc<Mutex<Vec<AxiomZProxy>>>
}

impl ProxyRotator {
    pub fn new() -> Self {
        Self {
            proxies: Arc::new(Mutex::new(Vec::new()))
        }
    }

    pub async fn get_proxy(&self) -> Result<reqwest::Proxy, Box<dyn std::error::Error + Send + Sync>> {
        let mut proxies_vec: Vec<&mut AxiomZProxy> = Vec::new();
        let mut weights: Vec<f64>  = Vec::new();

        let mut proxies = self.proxies.lock().await;
        for p in proxies.iter_mut() {
            if !p.is_available() {
                continue;
            }

            weights.push(p.score);
            proxies_vec.push(p);
        }

        let selected_idx: usize = {
            let dist = WeightedAliasIndex::new(weights)?;
            let mut rng = rand::rng();
            dist.sample(&mut rng)
        };

        proxies_vec[selected_idx].set_cooldown(tokio::time::Duration::from_secs(3));
        let proxy = {
            let proxy_url = format!("http://{}:{}", proxies_vec[selected_idx].ip, proxies_vec[selected_idx].port);
            reqwest::Proxy::http(proxy_url)?
        };
        
        return Ok(proxy)
    }

    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fetched_proxies = ProxyFetcher::fetch("http", 50).await?;
        ProxyFetcher::load(self.proxies.clone(), fetched_proxies).await?;

        // Spawn proxy refresh task
        let proxies_clone = self.proxies.clone();
        tokio::spawn(async move {
            ProxyFetcher::refresh_proxies(proxies_clone).await;
        });

        Ok(())
    }
}

// ----------------- PROXY FETCHER ----------------------
#[derive(Debug, Deserialize)]
struct GeoNodeResponse {
    data: Vec<GeoProxy>,
}

#[derive(Debug, Deserialize)]
struct GeoProxy {
    pub ip: String,
    pub port: String,

    #[serde(default)]
    pub speed: Option<f64>,

    #[serde(default)]
    pub uptime: Option<f64>,
}

struct ProxyFetcher;

impl ProxyFetcher {
    pub async fn fetch(proxy_scheme: &str, min_proxies: usize) -> Result<Vec<GeoProxy>, Box<dyn std::error::Error + Send + Sync>> {
        let mut proxies = Vec::new();
        let mut page = 1;
    
        while proxies.len() < min_proxies {
            let geo_url = format!("https://proxylist.geonode.com/api/proxy-list?limit=200&page={}&sort_by=lastChecked&sort_type=desc&protocols={}", page, proxy_scheme);
    
            let response = reqwest::get(&geo_url).await?;
            let geo_node_response: GeoNodeResponse = response.json().await?;
    
            if geo_node_response.data.is_empty() {
                break; // no more results available
            }
    
            proxies.extend(geo_node_response.data);
            page += 1;
        }
        proxies.truncate(min_proxies);
    
        Ok(proxies)
    }

    pub async fn load(proxies: Arc<Mutex<Vec<AxiomZProxy>>>, fetched_proxies: Vec<GeoProxy>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut proxies = proxies.lock().await;
        proxies.clear();

        for geo_proxy in fetched_proxies {
            let proxy_score = Self::caculate_score(geo_proxy.speed, geo_proxy.uptime);
            proxies.push(AxiomZProxy {
                ip: geo_proxy.ip,
                port: geo_proxy.port.parse().unwrap(),
                score: proxy_score,
                cooldown: None
            });
        }
        
        Ok(())
    }

    fn caculate_score(latency: Option<f64>, uptime: Option<f64>) -> f64 {
        let latency: f64 = latency.unwrap_or(100.0).max(1.0);
        let uptime: f64 = uptime.unwrap_or(50.0).clamp(0.0, 100.0);

        let score = (uptime / 100.0).powf(1.5) / (1.0 + latency.sqrt() / 10.0);
        return score
    }

    pub async fn refresh_proxies(proxies: Arc<Mutex<Vec<AxiomZProxy>>>) {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(30 * 60));
        ticker.tick().await; // Discard immediate tick

        
        loop {
            ticker.tick().await;

            // Fetch proxies
            let fetched_proxies = match Self::fetch("http", 200).await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to fetch proxies: {e}");
                    continue;
                }
            };
            
            // Load proxies
            match Self::load(proxies.clone(), fetched_proxies).await {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("Failed to load proxies: {e}");
                }
            }
            
        }
    }
}