use rand::seq::IndexedRandom;
use reqwest;

pub struct Crawler;
impl Crawler {
    pub fn crawl_site(url:&str) {

    }
}

struct ClientProxy {
    client: reqwest::Client
}
impl ClientProxy {
    pub fn new() -> Result<Self, std::io::Error> {
        let proxy = Self::get_proxy()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .proxy(proxy)
            .build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(Self {
            client: client
        })
    }

    fn get_proxy() -> Result<reqwest::Proxy, std::io::Error> {
        let proxies_vec = Self::read_proxies("http_proxies.txt")?;
        let random_proxy_url = proxies_vec.choose(&mut rand::thread_rng())
            .unwrap()
            .to_string();

        let proxy = reqwest::Proxy::http(&random_proxy_url)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        return Ok(proxy);
    }

    fn read_proxies(file_path: &str) -> Result<Vec<String>, std::io::Error> {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        return Ok(lines);
    }
}

/* 
struct ClientProxy {
    client: reqwest::Client,
    proxies: Vec<String>
}
impl ClientProxy {
    pub fn new() -> Result<Self, std::io::Error>{
        let http_proxies = Self::read_proxies("http_proxies.txt")?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        let mut client_proxy = Self {
            client: client,
            proxies: http_proxies
        };
        client_proxy.change_proxy();

        return Ok(client_proxy);
    }

    pub fn change_proxy(&mut self){
        let new_proxy = {
            let proxy_url = self.proxies.choose(&mut rand::thread_rng())
                .unwrap()
                .to_string();
            let proxy = reqwest::Proxy::http(&proxy_url);
            proxy.unwrap()
        };
            

        let new_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .proxy(new_proxy)
            .build()
            .unwrap();
        
        self.client = new_client;
    }
    
    fn read_proxies(file_path: &str) -> Result<Vec<String>, std::io::Error> {
        let content = std::fs::read_to_string(file_path)?;
        let lines: Vec<String> = content.lines().map(String::from).collect();
        return Ok(lines);
    }
}
    */