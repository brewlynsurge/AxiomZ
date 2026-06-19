use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use url::Url;

use crate::proxy_rotator::ProxyRotator;
use shared;
use spider_shared::database::AxiomZDatabase;


use crate::crawler_animator::{self, CrawlerAnimator};


// ----------------- AXIOMZ SCRAPER ----------------------
pub struct AxiomZScraper {
    database: Arc<Mutex<AxiomZDatabase>>,
    fetcher: Fetcher,
    processor: PageProcessor
}

impl AxiomZScraper {
    pub fn new(proxy_rotator: ProxyRotator, axiomz_database: AxiomZDatabase) -> Self {
        let (tx, rx) = mpsc::channel::<FetcherResult>(20);
        let database = Arc::new(Mutex::new(axiomz_database));
        
        let axiomz_fetcher = Fetcher {
            proxy_rotator: Some(proxy_rotator),
            fetcher_tx: tx
        };

        let axiomz_page_processor = PageProcessor {
            database: database.clone(),
            fetcher_rx: Some(rx)
        };
        
        AxiomZScraper {
            database: database.clone(),
            fetcher: axiomz_fetcher,
            processor: axiomz_page_processor
        }
    }

    pub async fn start(&mut self, stream: Arc<Mutex<TcpStream>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let fetcher_handler = self.fetcher.spawn(stream.clone()).await?;
        let processor_handler = self.processor.spawn(stream.clone()).await?;

        let (fetcher_result, processor_handler) = tokio::join!(fetcher_handler, processor_handler);
        fetcher_result??;
        processor_handler??;
        
        Ok(())
    }
}

// --------------------- FETCHER -------------------------
struct FetcherParameters {
    proxy_rotator: ProxyRotator,
    fetcher_tx: mpsc::Sender<FetcherResult>,
    stream: Arc<Mutex<TcpStream>>
}

struct FetcherResult {
    pub url: String,
    pub response: Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>
}


struct Fetcher {
    proxy_rotator: Option<ProxyRotator>,
    fetcher_tx: mpsc::Sender<FetcherResult>
}

impl Fetcher {
    pub async fn spawn(&mut self, stream: Arc<Mutex<TcpStream>>) -> Result<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>> {
        let params = FetcherParameters {
            proxy_rotator: self.proxy_rotator.take().unwrap(),
            fetcher_tx: self.fetcher_tx.clone(),
            stream: stream
        };

        let handler = tokio::spawn(async move {
            Self::fetch(params).await
        });

        
        Ok(handler)
    }

    async fn fetch(mut params: FetcherParameters) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            match Self::fetch_page(&mut params).await {
                Ok(_) => {},
                Err(e) => {eprintln!("Fetching service failed inside the loop: {e}")}
            }
        }
    }

    async fn fetch_page(params: &mut FetcherParameters) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let page_url = {
            let mut stream = params.stream.lock().await;
            shared::socket::send_data::<String>(&mut stream, &String::from("GET_URL")).await?;
            shared::socket::receive_data::<String>(&mut stream).await?
        };

        let proxy = params.proxy_rotator.get_proxy().await?;
        let get_page_result = Self::get_page(&page_url, proxy).await;
        
        let fetcher_result = FetcherResult {
            url: page_url,
            response: get_page_result
        };

        params.fetcher_tx.send(fetcher_result).await?;
        Ok(())
    }

    async fn get_page(page_url: &str, proxy: reqwest::Proxy) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
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
}

// ------------------- PAGE PROCESSOR ----------------------
struct ProcessorParameters {
    stream: Arc<Mutex<tokio::net::TcpStream>>,
    database: Arc<Mutex<AxiomZDatabase>>,
    fetcher_rx: mpsc::Receiver<FetcherResult>,
    animator: CrawlerAnimator
}


struct PageProcessor {
    database: Arc<Mutex<AxiomZDatabase>>,
    fetcher_rx: Option<mpsc::Receiver<FetcherResult>>
}

impl PageProcessor {
    pub async fn spawn(&mut self, stream: Arc<Mutex<TcpStream>>) -> Result<tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>, Box<dyn std::error::Error + Send + Sync>> {
        let processor_params = ProcessorParameters {
            stream: stream,
            database: self.database.clone(),
            fetcher_rx: self.fetcher_rx.take().unwrap(),
            animator: CrawlerAnimator::new()
        };

        let handler = tokio::spawn(async move {
            Self::process(processor_params).await
        });

        Ok(handler)
    }

    async fn process(mut params: ProcessorParameters) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        params.animator.initialize().await?;
        
        while let Some(fetched_result) = params.fetcher_rx.recv().await {
            let animator_instance = params.animator.create_instance(&fetched_result.url.clone()).await?;
            let page_compiler = PageCompiler {
                url: fetched_result.url,
                response: fetched_result.response,
                database: params.database.clone(),
                animator_instance: animator_instance
            };

            match page_compiler.compile_page().await {
                Ok(_) => {},
                Err(e) => {eprintln!("Compiling service failed inside the loop: {e}")}
            }
        }

        Ok(())
    }
}

// ------------------- PAGE COMPILER ----------------------
struct PageCompiler {
    url: String,
    response: Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>,
    database: Arc<Mutex<AxiomZDatabase>>,
    animator_instance: crawler_animator::AnimatorInstance
}

impl PageCompiler {
    pub async fn compile_page(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.animator_instance.to_processing().await?;

        let response = match self.handle_response_error().await? {
            Some(res) => res,
            None => return Ok(())
        };
        
        //let database_tx = {
        //    let database = self.database.lock().await;
        //    database.pool.begin().await?
        //};
        
        
        Ok(())
    }

    async fn handle_response_error(&self) -> Result<Option<&reqwest::Response>, Box<dyn std::error::Error + Send + Sync>> {
        match self.response.as_ref() {
            Ok(res) => {
                match res.status() {
                    reqwest::StatusCode::OK => {
                        return Ok(Some(res))
                    },
                    reqwest::StatusCode::UNAUTHORIZED => {
                        self.animator_instance.to_failure(Some(format!("The website is unauthorized (status code: 401)"))).await?;
                        return Ok(None)
                    }
                    reqwest::StatusCode::NOT_FOUND => {
                        self.animator_instance.to_failure(Some(format!("The website is not found (status code: 404)"))).await?;
                        return Ok(None)
                    },
                    status => {
                        self.animator_instance.to_failure(Some(format!("The website return with a https status code {status}"))).await?;
                        return Ok(None)
                    }
                }
            },
            Err(e) => {
                self.animator_instance.to_failure(Some(format!("{e}"))).await?;
                return Ok(None)
            }
        };
    }
}
/* 
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

        let mut crawler_animator = CrawlerAnimator::new();
        crawler_animator.initialize().await?;

        let mut counter = 1;
        while let Some(page_container) = rx.recv().await {
            let page_url = page_container.url.clone();

            let animator_instance = crawler_animator.create_instance(&page_url).await?;
            animator_instance.to_processing().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            animator_instance.to_saving().await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(3000)).await;
            if counter % 3 == 0 {
                animator_instance.to_failure(Some("The website blocked you".to_string())).await?;
            } else{
                animator_instance.to_success().await?;
            }

            counter += 1;
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

*/