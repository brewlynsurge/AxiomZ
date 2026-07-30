use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::select;
use std::ops::Add;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::Mutex;
use crossterm::style::Stylize;
use serde::{Serialize, Deserialize};
use crate::config;
use crate::socket;

// ----------------- CRAWLER API COMMANDS ----------------------
#[derive(Debug, Serialize, Deserialize)]
pub enum CrawlerAPICommand {
    GetUrl,
    UrlOk(String),
    HeartBeat
}


// ----------------- CRAWER GLOBALS ----------------------
pub struct CrawlerGlobals {
    pub active_crawler_count: Arc<Mutex<usize>>
}

pub static CRAWLER_GLOBALS: LazyLock<CrawlerGlobals> = LazyLock::new(|| {
    CrawlerGlobals {
        active_crawler_count: Arc::new(Mutex::new(0))
    }
});

// ----------------- CRAWLER API ----------------------
pub struct CrawlerAPI {
    pub tcp_stream: TcpStream
}

impl CrawlerAPI {
    pub async fn new_connection(spider_config: &config::SpiderConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let connection_addr = format!("{}:{}", spider_config.host, spider_config.port);
        let connection_stream = TcpStream::connect(&connection_addr).await?;

        Ok(Self {
            tcp_stream: connection_stream
        })
    }

    async fn send_command(&mut self, command: CrawlerAPICommand) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        socket::send_data::<CrawlerAPICommand>(&mut self.tcp_stream, &command).await
    }

    pub async fn init_heatbeat(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        loop {
            heartbeat_interval.tick().await;
            socket::send_data::<CrawlerAPICommand>(&mut self.tcp_stream, &CrawlerAPICommand::HeartBeat).await?;
        }
    }


    pub async fn send_connection_type(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        socket::send_data::<super::server_api::ServerConnectionTypes>(&mut self.tcp_stream, &super::server_api::ServerConnectionTypes::Crawler).await?;
        Ok(())
    }

    pub async fn handle_crawler(mut tcp_stream: TcpStream, addr: std::net::SocketAddr, database_config: &config::DatabaseConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Send database configuration
        socket::send_data::<config::DatabaseConfig>(&mut tcp_stream, database_config).await?;
        
        {
            let mut active_crawler_count = CRAWLER_GLOBALS.active_crawler_count.lock().await;
            *active_crawler_count += 1;
        }

        println!("Connected to crawler: {}", addr);

        loop {
            select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(40)) => {
                    tcp_stream.shutdown().await?;
                    
                    {
                        let mut active_crawler_count = CRAWLER_GLOBALS.active_crawler_count.lock().await;
                        *active_crawler_count -= 1;
                    }
                }
                command = socket::receive_data::<CrawlerAPICommand>(&mut tcp_stream) => {
                    let command = match command {
                        Ok(c) => c,
                        Err(e) => {
                            println!("{}: {}", "CommandReceiveError".red().bold(), e);
                            continue;
                        }
                    };

                    match command {
                        CrawlerAPICommand::GetUrl => { GetUrl::server_fn(&mut tcp_stream).await? }
                        CrawlerAPICommand::UrlOk(url) => { UrlOk::server_fn(&mut tcp_stream, url).await? }
                        CrawlerAPICommand::HeartBeat => {}
                    }
                }
            }
        }
    }

    pub async fn call_get_url(&mut self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.send_command(CrawlerAPICommand::GetUrl).await?;
        let received_url = GetUrl::crawler_fn(&mut self.tcp_stream).await?;

        Ok(received_url)
    }

    pub async fn call_url_ok(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.send_command(CrawlerAPICommand::UrlOk(url.to_string())).await?;
        Ok(())
    }
    
}

// ----------------- GET_URL COMMAND ----------------------
struct GetUrl;
impl GetUrl {
    pub async fn server_fn(tcp_stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = String::from("https://notdefined.com"); // TODO: Change
        println!("TODO: Change not defined");
        socket::send_data::<String>(tcp_stream, &url).await?;
        Ok(())
    }

    pub async fn crawler_fn(tcp_stream: &mut TcpStream) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let received_url = socket::receive_data::<String>(tcp_stream).await?;
        Ok(received_url)
    }
}

// ----------------- URL_OK COMMAND ----------------------
struct UrlOk;
impl UrlOk {
    pub async fn server_fn(tcp_stream: &mut TcpStream, url: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }
}