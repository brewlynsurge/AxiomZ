use crossterm::style::Stylize;
use tokio::net::TcpListener;
use shared::{self, CliError, raise_error};
use super::handler;

pub async fn handle_spider_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spider_server = Server::start().await?;
    println!("Starting Server...");

    spider_server.handle_tcp_connections().await?;
    Ok(())
}


// ------------- SPIDER SERVER ---------------------
pub struct Server {
    listener: TcpListener,
    pub host: String,
    pub port: u16
}

impl Server {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let spider_config = {
            let config_loader = shared::config::ConfigLoader::new()
                .resolve("SPIDER")
                .execute_resolves();
            shared::config::SpiderConfig::load(&config_loader)
        };
        
        // Starting server
        let tcp_listener = TcpListener::bind(format!("{}:{}", spider_config.host, spider_config.port)).await?;

        Ok(Self {
            listener: tcp_listener,
            host: spider_config.host,
            port: spider_config.port
        })
    }

    pub async fn handle_tcp_connections(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let (socket, addr) = self.listener.accept().await?;

            tokio::spawn(async move {
                match handler::ClientHandler::handle_client(socket, addr).await {
                    Ok(_) => {},
                    Err(e) => {
                        raise_error!(CrawlerError, "Error from the crawler of address {}: {}", addr.to_string().red(), e);
                    }
                };
            });
        }
    }
}