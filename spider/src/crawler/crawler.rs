use tokio::sync::Mutex;
use std::{io::Write, sync::Arc};
use crossterm::{cursor::{Hide, MoveTo, Show, position}, style::Stylize};
use shared;
use shared::spider_api::CrawlerAPI;
use crate::proxy_rotator::ProxyRotator;
use crate::axiomz_scraper::AxiomZScraper;
use spider_shared::database::AxiomZDatabase;

// ----------------- CRAWLER ----------------------
pub struct Crawler {
    crawler_api: Arc<Mutex<CrawlerAPI>>,
    scraper: AxiomZScraper
}

impl Crawler {
    pub async fn build() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        println!(" {} initializing system {} ", "--".cyan().bold(), "--".cyan().bold());

        // Load spider configuration
        let spider_config = {
            let config_loader = shared::config::ConfigLoader::new()
                .resolve("SPIDER")
                .execute_resolves();

            shared::config::SpiderConfig::load(&config_loader)
        };

        // Connect to server
        let mut crawler_api = Self::try_server_connection(&spider_config).await?;

        // Proxy Rotator
        let proxy_rotator = ProxyRotator::new();
        {
            print!("   {} Initiaizing Proxy Rotator: ", "->".cyan().bold());
            std::io::stdout().flush()?;
            
            proxy_rotator.initialize().await?;
            println!("{}", "done".green().bold());
        }
        
        // Initializing Database
        print!("   {} Connecting to database: ", "->".cyan().bold());
        std::io::stdout().flush()?;
        let database_config: shared::config::DatabaseConfig = shared::socket::receive_data(&mut crawler_api.tcp_stream).await?;
        let axiomz_database = AxiomZDatabase::connect(&database_config).await?;
        println!("{}", "done".green().bold());
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Initializing scrapper
        let aziomz_scraper = AxiomZScraper::new(proxy_rotator, axiomz_database);
        
        Ok(Self {
            crawler_api: Arc::new(Mutex::new(crawler_api)),
            scraper: aziomz_scraper
        })
    }
    
    async fn try_server_connection(spider_config: &shared::config::SpiderConfig,) -> Result<CrawlerAPI, Box<dyn std::error::Error + Send + Sync>> {
        let (animation_tx, animation_rx) = tokio::sync::watch::channel(true);

        // Spawn animation task
        tokio::spawn(async move {
            let mut terminal_out = std::io::stdout();
            
            match Self::connection_animation(&mut terminal_out, animation_rx).await {
                Ok(_) => {let _ = crossterm::queue!(terminal_out, Show);}
                Err(e) => {
                    let _ = crossterm::queue!(terminal_out, Show);
                    eprintln!("Spinner animation failed: {e}");
                }
            };
        });

        // Server connection loop
        loop {
            match CrawlerAPI::new_connection(&spider_config).await {
                Ok(mut crawler_api) => {
                    crawler_api.send_connection_type().await?;
                    let _ = animation_tx.send(false);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    println!("");
                    
                    return Ok(crawler_api);
                }
                Err(_) => {tokio::time::sleep(std::time::Duration::from_secs(1)).await;}
            };
        }
    }

    async fn connection_animation(terminal_out: &mut std::io::Stdout, animation_rx: tokio::sync::watch::Receiver<bool>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (_, start_row) = position()?;
        crossterm::queue!(terminal_out, Hide)?;

        // Terminal connection animation
        let frames = ['|', '/', '-', '\\'];
        'animation_loop: loop {
            for f in frames.iter() {
                crossterm::queue!(terminal_out, MoveTo(0, start_row))?;
                write!(terminal_out, "   {} Connecting to server {f}", "->".cyan().bold())?;
                terminal_out.flush()?;
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                if !*animation_rx.borrow() {
                    crossterm::queue!(terminal_out, MoveTo(0, start_row))?;
                    write!(terminal_out, "   {} Connecting to server: {}", "->".cyan().bold(), "done".green().bold())?;
                    terminal_out.flush()?;
                    break 'animation_loop Ok(())
                }
            }
        }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.scraper.start(self.crawler_api.clone()).await
    }
}
