use tokio::net::TcpStream;
use std::io::Write;
use crossterm::{cursor::{Hide, MoveTo, Show, position}, style::Stylize};
use shared;

// ----------------- CRAWLER ----------------------
pub struct Crawler;

impl Crawler {
    pub async fn build() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        println!(" -- initializing system -- ");

        // Load spider configuration
        let spider_config = {
            let config_loader = shared::config::ConfigLoader::new()
                .resolve("SPIDER")
                .execute_resolves();

            shared::config::SpiderConfig::load(&config_loader)
        };

        // Connect to server
        Self::try_server_connection(&spider_config).await?;

        todo!()
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
}
