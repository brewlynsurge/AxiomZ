use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use std::sync::Arc;
use reqwest::{Proxy, Client};

use crate::utils;

/*
SurfXProxy
*/
#[derive(Debug, Clone)]
pub struct SurfXProxy {
    pub url: String
}

impl SurfXProxy {
    pub fn new(url: &str) -> Self {
        Self { url: url.to_string() }
    }
}

/*
ProxyRotator
*/
pub struct ProxyRotator {
    valid_proxies: Arc<Mutex<Vec<SurfXProxy>>>
}

impl ProxyRotator {
    pub fn new() -> Self {
        let proxy_rotator = Self {
            valid_proxies: Arc::new(Mutex::new(Vec::new()))
        };

        // Spawn proxy validation thread
        let valid_proxies_clone = proxy_rotator.valid_proxies.clone();
        thread::spawn(|| {
            // Create a new runtime inside the thread
            let rt = Runtime::new().unwrap();
            rt.block_on(Self::validate_proxies(valid_proxies_clone));
        });

        return proxy_rotator;
    }

    pub async fn get_proxy(&self) -> String {
        let tries = 5;

        for _ in 0..tries {
            let mut valid_proxies = self.valid_proxies.lock().await;
            match valid_proxies.get(0) {
                Some(_) => {
                    // Swapping the first proxy to last of the vector
                    let surfx_proxy = valid_proxies.remove(0);
                    valid_proxies.push(surfx_proxy.clone());
                    
                    return surfx_proxy.url;
                },
                None => { tokio::time::sleep(tokio::time::Duration::from_millis(500)).await }
            }
        }
        
        panic!("Most of the proxies are not functional. Kindly change the proxies list.");
    }

    async fn validate_proxies(valid_proxies: Arc<Mutex<Vec<SurfXProxy>>>) {
        let proxies: Vec<String> = utils::PROXY_FILE.lines().map(String::from).collect();
        let batch_size = 10;
        

        for chunk in proxies.chunks(batch_size) {
            let mut handles = vec![];

            // Creating async handle
            for proxy_url in chunk.iter() {
                let proxy_url = proxy_url.clone();

                let handle = tokio::spawn(async move {
                    Self::test_proxy(proxy_url).await
                });
                handles.push(handle);
            }

            // Joining async handles
            for handle in handles {
                match handle.await.unwrap() {
                    Ok(proxy_url) => {
                        let mut proxies = valid_proxies.lock().await;
                        proxies.push(SurfXProxy::new(&proxy_url));
                        println!("{}", proxies.len())
                    },
                    Err(_) => {}
                }
            }
        }
    }

    async fn test_proxy(proxy_url: String) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let proxy = Proxy::all(&proxy_url)?;
        let client = Client::builder()
            .proxy(proxy)
            .timeout(std::time::Duration::from_secs(5))
            .build()?;

        let response = client.get("http://httpbin.org/ip").send().await?;
        if  response.status().is_success() {
            Ok(proxy_url)
        } else {
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Proxy returned status: {}", response.status()))))
        }
    }

}