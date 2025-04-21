#![allow(unused)]

use std::collections::HashSet;
use rand::seq::IndexedRandom;
use reqwest;
use scraper;
use url;
use whatlang;
use unidecode;
use rust_stemmers;
use robotstxt;
use super::utils::Console;

pub struct Crawler;
impl Crawler {
    pub async fn crawl_site(url:&str) -> Result<(String, String, HashSet<String>, HashSet<String>), std::io::Error>{
        // Checking if the url can be crawled
        let is_scraping_allowed = Self::is_scraping_allowed(url, "SurfXSpiderRobot").await?;
        if !is_scraping_allowed {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "The site is not allowed to crawl"));
        }
        
        let client = ClientProxy::new("http_proxies.txt").await?;

        let response = client.get(url)
            .send()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let response_text = response.text().await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let document = scraper::Html::parse_document(&response_text);

        let title = Self::get_title(&document)?;
        if title.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "The page doesn't have a title"));
        }

        let description = Self::get_description(&document)?;
        let all_links = Self::get_all_links(&document)?;
        let all_words = Self::get_all_words(&document)?;

        Ok((title, description, all_words, all_links))
    }

    fn get_title(document: &scraper::Html) -> Result<String, std::io::Error> {
        let title_selector = scraper::Selector::parse("title")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
        let title = document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(""))
            .unwrap_or_default();
        return Ok(title);
    }

    fn get_description(document: &scraper::Html) -> Result<String, std::io::Error> {
        let meta_selector = scraper::Selector::parse(r#"meta[name="description"]"#)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        let description = document
            .select(&meta_selector)
            .next()
            .and_then(|el| el.value().attr("content"))
            .unwrap_or_default()
            .to_string();
        return Ok(description);
    }

    fn get_all_links(document: &scraper::Html) -> Result<HashSet<String>, std::io::Error> {
        let mut all_links = HashSet::new();
        let selector = scraper::Selector::parse("a")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        for element in document.select(&selector) {
            if let Some(link) = element.value().attr("href") {
                if url::Url::parse(link).is_ok() {
                    all_links.insert(link.to_string());
                }
            }
        }

        return Ok(all_links);
    }

    fn get_all_words(document: &scraper::Html) -> Result<HashSet<String>, std::io::Error> {
        let selector = scraper::Selector::parse("p, div, span, li, h1, h2, h3, h4, h5, h6")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        let stop_words = [
            "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is",
            "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there",
            "these", "they", "this", "to", "was", "will", "with",
        ].iter().map(|s| s.to_string()).collect::<Vec<String>>();
        
        let mut all_words = HashSet::new();
        for element in document.select(&selector) {
            let tag_name = element.value().name();
            if matches!(tag_name, "script" | "style" | "head" | "noscript" | "meta" | "svg" | "link") {
                continue;
            }

            for node in element.text() {
                let mut text = node.trim().to_string();
                if text.is_empty() {continue;}
                
                // Skip obvious garbage
                let is_garbage = {
                    let obvious_garbages = ["http", ".com", ".png", ".jpg", ".svg", "color", "font", "px", "{"];
                    let mut is_garbage = false;
                    for i in obvious_garbages {
                        if text.contains(i) {
                            is_garbage = true;
                            break;
                        }
                    }
                    is_garbage
                };
                if is_garbage {continue;}

                let is_english = whatlang::detect(&text).map_or(false, |info| info.lang() == whatlang::Lang::Eng);
                if !is_english {continue;}

                if !text.is_ascii() {
                    text = unidecode::unidecode(&text);
                }

                let words = {
                    let english_stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);
                    let mut words = Vec::new();
                    for word in text.split_whitespace() {
                        let word = word.chars()
                            .filter(|c| c.is_alphanumeric())
                            .collect::<String>()
                            .replace(" ", "")
                            .to_lowercase();
                        let stemmerized_word = english_stemmer.stem(&word)
                            .to_string();
                        
                        if stop_words.contains(&word) || word.is_empty() {
                            continue;
                        }

                        words.push(word);
                    }

                    words
                };
                
                if !words.is_empty() {
                    all_words.extend(words);
                }
            }
        }

        Ok(all_words)
    }

    async fn is_scraping_allowed(url: &str, user_agent: &str) -> Result<bool, std::io::Error> {
        let parsed_url = reqwest::Url::parse(url)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let host_str = parsed_url.host_str().ok_or("Invalid host")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        let robots_url = format!("{}://{}{}", parsed_url.scheme(), host_str, "/robots.txt");
        let response = reqwest::get(&robots_url).await;
        
        let robots_txt = match response {
            Ok(resp) => {
                if resp.status().is_success() {
                    resp.text().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
                } else {return Ok(true)}
            },
            Err(_) => {return Ok(true);}
        };

        let mut matcher = robotstxt::DefaultMatcher::default();
        Ok(matcher.one_agent_allowed_by_robots(&robots_txt, user_agent, url))
    }
}


struct ClientProxy;
impl ClientProxy {
    pub async fn new(file_path: &str) -> Result<reqwest::Client, std::io::Error> {
        let proxies_vec = Self::read_proxies(file_path)?;

        let client_proxy = loop {
            let random_url = proxies_vec.choose(&mut rand::rng())
                .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Failed to get a random url from the vector"))?
                .to_string();
            let proxy = reqwest::Proxy::http(&random_url)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;


            let client = reqwest::Client::builder()
                .proxy(proxy.clone())
                .connect_timeout(std::time::Duration::from_secs(5))  // 1 s connect timeout
                .timeout(std::time::Duration::from_secs(5))          // 1 s overall timeout
                .build()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            
            let is_alive = match client.get("https://httpbin.org/ip").send().await {
                Ok(resp) => {resp.status().is_success()},
                Err(e) => {false}
            };
            
            if is_alive {
                break proxy;
            } else {
                Console::error(&format!("Failed to connect to proxy: {}", random_url), Some("client_proxy"));
            }
            
        };

        let client = reqwest::Client::builder()
            .proxy(client_proxy)
            .build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(client)
    }

    fn read_proxies(file_path: &str) -> Result<Vec<String>, std::io::Error> {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        return Ok(lines);
    }
}