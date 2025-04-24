#![allow(unused)]

use std::collections::HashSet;
use rand::seq::IndexedRandom;
use reqwest;
use scraper;
use url;
use rust_stemmers;
use robotstxt;
use regex::Regex;
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
        let all_links = Self::get_all_links(&document, 4)?;
        let all_text = Self::get_all_texts(&document)?;

        Ok((title, description, all_text, all_links))
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

    fn get_all_links(document: &scraper::Html, link_path_max_lenght: usize) -> Result<HashSet<String>, std::io::Error> {
        let mut all_links = HashSet::new();
        let selector = scraper::Selector::parse("a")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        for element in document.select(&selector) {
            if let Some(link) = element.value().attr("href") {
                if let Ok(url) = url::Url::parse(link) {
                    if url.scheme() == "http" || url.scheme() == "https" {
                        let url_path = url.path();
                        if url_path.contains("%") {
                            continue;
                        }

                        let slash_count = url_path.chars().filter(|&c| c == '/').count();
                        if slash_count <= link_path_max_lenght {
                            all_links.insert(url.to_string());
                        }
                    }
                }

            }

        }

        return Ok(all_links);
    }

    fn get_all_texts(document: &scraper::Html) -> Result<HashSet<String>, std::io::Error> {
        let mut all_text = HashSet::new();
        for text_node in document.root_element().text() {
            all_text.insert(text_node.to_string());
        }
        
        Ok(all_text)
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
                .connect_timeout(std::time::Duration::from_secs(10))
                .timeout(std::time::Duration::from_secs(30))
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