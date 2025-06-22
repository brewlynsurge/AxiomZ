#![allow(unused)]

use std::collections::HashSet;
use rand::seq::IndexedRandom;
use reqwest;
use scraper;
use url;
use rust_stemmers;
use texting_robots::Robot;
use regex::Regex;
use super::utils::Console;
use super::utils;

pub struct Crawler {
    user_agent: String,
    proxies_list: Vec<String>
}


impl Crawler {
    pub fn new(user_agent: &str) -> Self {
        let proxies = utils::PROXY_FILE.lines().map(String::from).collect();

        Self {
            user_agent: user_agent.to_string(),
            proxies_list: proxies
        }
    }

    pub async fn crawl_site(&self, url:&str) -> Result<(String, String, HashSet<String>, HashSet<String>), Box<dyn std::error::Error>>{    
        // Checking if the url can be crawled
        if !Self::is_scraping_allowed(url, "SurfXSpiderRobot").await? {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("The site is not allowed to crawl"))));
        }

        async fn try_scrapping(site_url: &str, proxy_url: &str) -> Result<String, Box<dyn std::error::Error>> {
            let proxy = reqwest::Proxy::all(proxy_url)?;
            let client = reqwest::Client::builder()
                .proxy(proxy)
                .timeout(std::time::Duration::from_secs(30))
                .build()?;
            
            let response = client.get(site_url)
                .send()
                .await?;

            let response_text = response.text().await?;
            

            Ok(response_text)
        }
        
        println!("Scrapping...");
        let mut retry_index = 0;
        let mut response_data = None;
        while retry_index <= 3 {
            let random_proxy = self.proxies_list.choose(&mut rand::rng()).unwrap().as_str();
            let response = try_scrapping(url, random_proxy).await;

            if response.is_ok(){
                response_data = Some(response.unwrap());
                break;
            }
            retry_index += 1;
            println!("Retry: {}", retry_index);
        }
        
        println!("response_data: {:?}", response_data);
        /* 
        let mut retry_index = 0;
        let response: String = while retry_index <= 3 {
            let random_proxy = self.proxies_list.choose(&mut rand::rng()).unwrap();
            let proxy = reqwest::Proxy::all(random_proxy)?;
            
            let client = match reqwest::Client::builder()
                .proxy(proxy)
                .build() {
                    Ok(c) => c,
                    Err(_) => {
                        // TODO: println!("Error building client");
                        retry_index += 1;
                        continue;
                    }
                };

            let response = match client.get(url)
                .send()
                .await {
                    Ok(r) => r,
                    Err(_) => {
                        // TODO: println!("Error sending request");
                        retry_index += 1;
                        continue;
                    }
                };
           
            let response_text = response.text().await;
            if response_text.is_ok() {
                break response_text.unwrap();
            }
        
        };
        */

        /*
        let client = ClientProxy::new().await?;

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
         */
        todo!()
    }

    async fn fetch_url() {
        todo!()
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

    async fn is_scraping_allowed(url: &str, user_agent: &str) -> Result<bool, Box<dyn std::error::Error>> {
        // Parse the input URL
        let url = url::Url::parse(url)?;
        let robots_url = format!("{}://{}/robots.txt", url.scheme(), url.host_str().ok_or("The host is invalid")?);

        // Fetch the robots.txt content
        let response = reqwest::get(&robots_url).await;
        let robots_txt = match response {
            Ok(resp) => resp.text().await?,
            Err(_) => return Ok(true) // If robots.txt is not found or inaccessible, assume crawling is allowed
        };

        // Parse the robots.txt content
        let robots = Robot::new(user_agent, robots_txt.as_bytes())?;
        
        let path = url.path();
        Ok(robots.allowed(path))
    }
}

/*
ClientProxy
*/
struct ClientProxy {
    proxies_list: Vec<String>
}

impl ClientProxy {
    pub fn new() -> Self {
        let proxies = utils::PROXY_FILE.lines().map(String::from).collect();

        Self { proxies_list: proxies }
    }

    pub async fn new_client(&self) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
        let client_proxy = self.get_random_proxy().await;
        let client = reqwest::Client::builder()
            .proxy(client_proxy)
            .build()?;

        Ok(client)
    }

    async fn get_random_proxy(&self) -> reqwest::Proxy {
        let mut rng = rand::rng();

        loop {
            let random_proxy = self.proxies_list.choose(&mut rand::rng());
            if random_proxy.is_some() {
                let proxy = random_proxy.unwrap();
                if self.check_proxy(proxy).await {
                   return reqwest::Proxy::all(proxy).unwrap();
                } 
            }
            println!("No proxy found");
        }
    }

    async fn check_proxy(&self, proxy: &str) -> bool {
        let proxy = match reqwest::Proxy::all(proxy) {
            Ok(p) => p,
            Err(_) => return false,
        };

        let client = match reqwest::Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build() {
                Ok(c) => c,
                Err(_) => return false,
            };

        let res = client
            .head("https://httpbin.org/ip")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
        
        println!("res: {:?}", res);

        res.is_ok()
    }
}