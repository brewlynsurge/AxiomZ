use crossterm::style::Stylize;
use crossterm::{cursor, queue};
use std::io::Write;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use url::Url;

use crate::proxy_rotator::ProxyRotator;
use shared;
use spider_shared::database::AxiomZDatabase;


use crate::crawler_animator::Animator;

// ----------------- SCRAPER ----------------------
pub struct Scraper {
    proxy_rotator: Arc<Mutex<ProxyRotator>>,
    database: Arc<Mutex<AxiomZDatabase>>,
}

impl Scraper {
    pub fn new(proxy_rotator: ProxyRotator, axiomz_database: AxiomZDatabase) -> Self {
        Self {
            proxy_rotator: Arc::new(Mutex::new(proxy_rotator)),
            database: Arc::new(Mutex::new(axiomz_database)),
        }
    }

    pub async fn handle_scraper_task(&self, stream: Arc<Mutex<TcpStream>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = mpsc::channel::<PageContainer>(20);

        // Spawing Scraper task
        let stream_clone = stream.clone();
        let proxy_rotator = self.proxy_rotator.clone();
        let scrape_page_task = tokio::spawn(async move {
            Self::hande_scrape_page_task(tx, stream_clone, proxy_rotator).await
        });

        // Spawing Processing task
        let database_clone = self.database.clone();
        let process_page_task = tokio::spawn(async move { Self::handle_process_page_task(rx, database_clone).await });

        // Spawn results
        let (_, process_result) = tokio::join!(scrape_page_task, process_page_task);
        process_result??;

        Ok(())
    }

    async fn hande_scrape_page_task(tx: mpsc::Sender<PageContainer>, stream: Arc<Mutex<TcpStream>>, proxy_rotator: Arc<Mutex<ProxyRotator>>) {
        let mut proxy_rotator = proxy_rotator.lock().await;

        loop {
            match Self::scrape_page(tx.clone(), &mut proxy_rotator, stream.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error in scraping page: {e}")
                }
            }
        }
    }

    async fn scrape_page(tx: mpsc::Sender<PageContainer>, proxy_rotator: &mut tokio::sync::MutexGuard<'_, ProxyRotator>, stream: Arc<Mutex<TcpStream>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let page_url = {
            let mut stream = stream.lock().await;
            shared::socket::send_data::<String>(&mut stream, &String::from("GET_URL")).await?;
            shared::socket::receive_data::<String>(&mut stream).await?
        };

        let proxy = proxy_rotator.get_proxy().await?;

        let res = Self::fetch_page(&page_url, proxy).await;
        let pg_container = PageContainer {
            url: page_url,
            response: res,
        };

        tx.send(pg_container).await?;

        Ok(())
    }

    async fn fetch_page(page_url: &str, proxy: reqwest::Proxy) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
        let client = reqwest::Client::builder()
            .proxy(proxy)
            .timeout(tokio::time::Duration::from_secs(40))
            .connect_timeout(tokio::time::Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;

        let response = client
            .get(page_url)
            .header("Accept", "text/html")
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36",)
            .send()
            .await?;

        Ok(response)
    }

    async fn handle_process_page_task(mut rx: mpsc::Receiver<PageContainer>, database: Arc<Mutex<AxiomZDatabase>>, ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let database = database.lock().await;
        let tx = database.pool.begin().await?;

        let mut crawler_animator = Animator::new();
        crawler_animator.print_head()?;
        
        while let Some(page_container) = rx.recv().await {
            let page_url = page_container.url.clone();

            let mut crawler_animator = crawler_animator.process(&page_url).await?;
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            crawler_animator.to_saving_mode().await?;

            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let mut crawler_animator = crawler_animator.success(&page_url).await?;
            //let mut crawler_animator = crawler_animator.failure(&page_url, Some("The site blocked the scrapper, error 404".to_string())).await?;

            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let mut crawler_animator = crawler_animator.reset().await;
            
        }

        Ok(())
    }

    async fn process_page() {

    }
}

// ----------------- PAGE CONTAINER ----------------------
#[derive(Debug)]
struct PageContainer {
    pub url: String,
    pub response: Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>,
}