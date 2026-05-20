use tokio::net::{TcpListener, TcpStream};
use shared;

pub async fn handle_spider_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spider_server = SpiderServer::start().await?;
    println!("Server started...");
    
    spider_server.handle_tcp_clients().await?;
    
    
    Ok(())
}

// ------------- SPIDER SERVER ---------------------
pub struct SpiderServer {
    listener: TcpListener,
    pub host: String,
    pub port: u16
}

impl SpiderServer {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let spider_conif = shared::SpiderConfig::load()?;

        // Starting server
        let tcp_listener = TcpListener::bind(format!("{}:{}", spider_conif.host, spider_conif.port)).await?;
        
        Ok(Self {
            listener: tcp_listener,
            host: spider_conif.host,
            port: spider_conif.port
        })
    }

    pub async fn handle_tcp_clients(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        todo!()

        Ok(())
    }
    
}