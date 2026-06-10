use tokio::{net::TcpStream, sync::Mutex};
use std::{io::Write, sync::Arc};
use crossterm::{cursor::{Hide, MoveTo, Show, position}, style::Stylize};
use shared;
use crate::proxy_rotator::ProxyRotator;
use crate::scraper;
use spider_shared::database::AxiomZDatabase;

// ----------------- CRAWLER ----------------------
pub struct Crawler {
    stream: Arc<Mutex<TcpStream>>,
    pub scraper: scraper::Scraper
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
        let mut socket = Self::try_server_connection(&spider_config).await?;

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
        let database_config: shared::config::DatabaseConfig = shared::socket::receive_data(&mut socket).await?;
        let axiomz_database = AxiomZDatabase::connect(&database_config).await?;
        println!("{}", "done".green().bold());
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        // Initializing scrapper
        let aziomz_scraper = scraper::Scraper::new(proxy_rotator, axiomz_database);
        
        Ok(Self {
            stream: Arc::new(Mutex::new(socket)),
            scraper: aziomz_scraper
        })
    }
    
    async fn try_server_connection(spider_config: &shared::config::SpiderConfig,) -> Result<TcpStream, Box<dyn std::error::Error + Send + Sync>> {
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
        let connection_addr = format!("{}:{}", spider_config.host, spider_config.port);
        loop {
            match TcpStream::connect(&connection_addr).await {
                Ok(socket) => {
                    let _ = animation_tx.send(false);
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    println!("");
                    
                    return Ok(socket);
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
        self.scraper.handle_scraper_task(self.stream.clone()).await
    }
}
