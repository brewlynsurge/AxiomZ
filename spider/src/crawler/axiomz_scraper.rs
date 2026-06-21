use std::sync::Arc;
use reqwest::Response;
use tokio::net::TcpStream;
use tokio::sync::{Mutex, mpsc};
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use rust_stemmers;

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
            let mut page_compiler = PageCompiler {
                url: fetched_result.url,
                response: Some(fetched_result.response),
                database: params.database.clone(),
                animator_instance: animator_instance,
                stemmer: rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English)
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
static STOP_WORDS: LazyLock<Vec<&str>> = LazyLock::new(|| {
    include_str!("./../../stopwords.txt")
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
});


struct PageCompiler {
    url: String,
    response: Option<Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>>>,
    database: Arc<Mutex<AxiomZDatabase>>,
    animator_instance: crawler_animator::AnimatorInstance,
    stemmer: rust_stemmers::Stemmer
}

impl PageCompiler {
    pub async fn compile_page(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.animator_instance.to_processing().await?;

        let response = match self.handle_response_error().await? {
            Some(res) => res,
            None => return Ok(())
        };
        self.extract_page(response).await?;
        
        
        Ok(())
    }

    async fn handle_response_error(&mut self) -> Result<Option<reqwest::Response>, Box<dyn std::error::Error + Send + Sync>> {
        match self.response.take().unwrap() {
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

    async fn extract_page(&self, response: Response) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let html = response.text().await?;
        let document = Html::parse_document(&html);

        let page_title = {
            let title = Selector::parse("title").unwrap();
            document
                .select(&title)
                .next()
                .map(|e| e.text().collect::<String>())
                .unwrap_or_else(|| "Untitled".to_string())
        };

        let words = self.filter_out_words(&document);
        let links = UrlFiter::filter(&self.url, &document)?;

        drop(document);
        
        Ok(())
    }

    fn filter_out_words(&self, document: &Html) -> HashMap<String, usize> {
        let mut content_holder:HashMap<String, usize> = HashMap::new();

        let content_selector = Selector::parse("p,h1,h2,h3,h4,h5,h6,li,blockquote").unwrap();
        for element in document.select(&content_selector) {
            let text = element.text().collect::<Vec<_>>().join(" ");

            for word in text.split_whitespace() {
                // Filter word and skip other languages
                if !word.is_ascii() { continue; }
                
                let word: String = word
                    .chars()
                    .filter(|c| c.is_ascii_alphabetic())
                    .flat_map(|c| c.to_lowercase())
                    .collect();
                if word.is_empty() {continue;}

                if STOP_WORDS.contains(&word.as_str()) { continue;} // Remove stop words
                // Stemmerize word
                let word = self.stemmer.stem(&word).to_string();

                // Store word in content_holder
                if let Some(count) = content_holder.get(&word) {
                    content_holder.insert(word, count + 1);
                } else {
                    content_holder.insert(word, 1);
                }
            }
            
        }
        

        return content_holder;
    }

}


// ------------------- URL FILTER ----------------------
struct UrlFiter;

impl UrlFiter {
    const URL_PARAMETER_BLACKLIST: [&str;14] = [
        "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content", "fbclid", "gclid",
        "msclkid", "session", "sessionid", "sid", "phpsessid", "token", "auth"
    ];

    const URL_EXTENSION_BLACKLIST: [&str;28] = [
        "jpg", "jpeg", "png", "gif", "webp", "svg", "mp4", "js", "ppt", "pptx", "tar", "gz", "exe", "msi",
        "avi", "mov", "zip", "rar", "7z", "pdf", "css", "ico", "doc", "docx", "bz2", "apk", "iso", "dmg"
    ];
    
    pub fn filter(base_url: &str, document: &Html) -> Result<HashSet<String>, Box<dyn std::error::Error + Send + Sync>> {
        let base_url = url::Url::parse(base_url)?;
        let selector = Selector::parse("a").unwrap();

        let mut filtered_links = HashSet::new();
        for element in document.select(&selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(abs_url) = base_url.join(href) {
                    if abs_url.scheme() == "http" || abs_url.scheme() == "https" {
                        let mut fetched_url = abs_url;
                        fetched_url.set_fragment(None); // Removing '#' sections
                        Self::filter_default_ports(&mut fetched_url); // Removing default ports

                        // Filter out parameters which are in blacklist
                        Self::filter_parameters(&mut fetched_url);

                        // Filter media files
                        let url_path = std::path::Path::new(fetched_url.path());
                        if let Some(path_extension) = url_path.extension() {
                            let path_extension = path_extension.to_string_lossy().to_ascii_lowercase();
                            if Self::URL_EXTENSION_BLACKLIST.contains(&path_extension.as_str()) {
                                continue;
                            }
                        }
                        
                        // Skip Very long urls
                        if fetched_url.as_str().len() > 2048 {
                            continue;
                        }

                        // Skip some url path traps
                        /*
                         * This fiteration is little too aggressive
                         * In later versions change it
                         */
                        if let Some(path_str) = url_path.to_str() {
                            if path_str.contains("/search") || path_str.contains("/find") || path_str.contains("/query") || path_str.contains("/results") {
                                continue;
                            }
                        }

                        // Skip url's containing usernames (rare but good to skip)
                        if !fetched_url.username().is_empty() {
                            continue;
                        }
                        
                        filtered_links.insert(fetched_url.to_string());
                    }
                }
            }
        }
        
        Ok(filtered_links)
    }

    fn filter_default_ports(fetched_url: &mut url::Url) {
        match (fetched_url.scheme(), fetched_url.port()) {
            ("http", Some(80)) => {
                let _ = fetched_url.set_port(None);
            }
            ("https", Some(443)) => {
                let _ = fetched_url.set_port(None);
            }
            _ => {}
        }
    }

    fn filter_parameters(fetched_url: &mut url::Url) {
        let filtered_params: Vec<_> = fetched_url
            .query_pairs()
            .filter(|(k, _)| {
                !Self::URL_PARAMETER_BLACKLIST
                    .iter()
                    .any(|blocked| blocked.eq_ignore_ascii_case(k))
            })
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect::<Vec<_>>();
        fetched_url.set_query(None);
        
        if !filtered_params.is_empty() {
            let mut query_params = fetched_url.query_pairs_mut();
            for (k, v) in filtered_params {
                query_params.append_pair(&k, &v);
            }
        }
    }
}