use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use std::sync::Arc;

use crate::proxy_rotator::ProxyRotator;
use shared;

// ----------------- SCRAPER ----------------------
pub struct Scraper {
    proxy_rotator: Arc<Mutex<ProxyRotator>>
}

impl Scraper {
    pub fn new(proxy_rotator: ProxyRotator) -> Self {
        Self {
            proxy_rotator: Arc::new(Mutex::new(proxy_rotator))
        }
    }

    pub async fn handle_scraper_task(&self, stream: Arc<Mutex<TcpStream>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = mpsc::channel::<Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>>(20);
        
        // Spawing Scraper task
        let stream_clone = stream.clone();
        let proxy_rotator = self.proxy_rotator.clone();
        let scrape_page_task = tokio::spawn(async move {
            Self::hande_scrape_page_task(tx, stream_clone, proxy_rotator).await
        });

        // Spawing Processing task
        let stream_clone = stream.clone();
        let process_page_task = tokio::spawn(async {
            Self::handle_process_page_task(rx, stream_clone).await
        });

        // Spawn results
        let (_, process_result) = tokio::join!(scrape_page_task, process_page_task);
        process_result??;

        Ok(())
    }

    async fn hande_scrape_page_task(tx: mpsc::Sender<Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>>, stream: Arc<Mutex<TcpStream>>, proxy_rotator: Arc<Mutex<ProxyRotator>>) {
        let mut proxy_rotator = proxy_rotator.lock().await;
        
        loop {
            match Self::scrape_page(tx.clone(), &mut proxy_rotator, stream.clone()).await {
                Ok(_) => {},
                Err(e) => {eprintln!("Error in scraping page: {e}")}
            }
            
        }
    }

    async fn scrape_page(tx: mpsc::Sender<Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>>, proxy_rotator: &mut tokio::sync::MutexGuard<'_, ProxyRotator>, stream: Arc<Mutex<TcpStream>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let page_url = {
            let mut stream = stream.lock().await;
            shared::socket::send_data::<String>(&mut stream, &String::from("GET_URL")).await?;
            shared::socket::receive_data::<String>(&mut stream).await?
        };

        let proxy = proxy_rotator.get_proxy().await?;
        let res = Self::fetch_page(&page_url, proxy).await;
        tx.send(res).await?;
        
        Ok(())
    }

    async fn fetch_page(page_url: &str, proxy: reqwest::Proxy) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(tokio::time::Duration::from_secs(40))
            .connect_timeout(tokio::time::Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        let response = client.get(page_url)
            .header("Accept", "text/html")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .send()
            .await?;
            
        Ok(response)
    }

    async fn handle_process_page_task(mut rx: mpsc::Receiver<Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>>, stream: Arc<Mutex<TcpStream>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        while let Some(response) = rx.recv().await {
            println!("Processing {:?}", response.is_ok());
        }


        Ok(())
    }

    
    
}
