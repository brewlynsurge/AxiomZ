use crossterm::style::Stylize;
use serde::{Serialize, Deserialize};
use tokio::net::TcpListener;
use crate::config;
use crate::socket;


// ----------------- SPIDER SERVER API ----------------------
pub struct ServerAPI {
    tcp_listener: TcpListener
}

impl ServerAPI {
    pub async fn build(spider_config: &config::SpiderConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let tcp_listener = TcpListener::bind(format!("{}:{}", spider_config.host, spider_config.port)).await?;
        
        Ok(Self {
            tcp_listener: tcp_listener
        })
    }

    pub async fn handle_connections(&self, database_config: &config::DatabaseConfig) {
        loop {
            let (mut socket, addr) = match self.tcp_listener.accept().await {
                Ok(n) => n,
                Err(e) => {
                    println!("{}: {}", "TcpConnectionError".red().bold(), e);
                    continue;
                }
            };

            let connection_type = match socket::receive_data::<ServerConnectionTypes>(&mut socket).await {
                Ok(c) => c,
                Err(e) => {
                    println!("{}: {}", "TcpConnectionInitializationError".red().bold(), e);
                    continue;
                }
            };

            let database_config = database_config.clone();
            tokio::spawn(async move {
                match connection_type {
                    ServerConnectionTypes::Crawler => {super::CrawlerAPI::handle_crawler(socket, addr, &database_config)},
                    ServerConnectionTypes::Client => {
                        todo!()
                    }
                }.await
            });
        }
    }
}

// ----------------- SERVER CONNECTION TYPES ----------------------
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerConnectionTypes {
    Crawler,
    Client
}