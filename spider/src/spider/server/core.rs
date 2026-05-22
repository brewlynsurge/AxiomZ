use tokio::net::{TcpListener, TcpStream};
use shared;

pub async fn handle_spider_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spider_server = Server::start().await?; // TODO
    println!("Starting Server...");
    todo!();
    loop {
        
    }
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

    pub async fn handle_tcp_clients(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            // Accept new connection
            let (socket, addr) = self.listener.accept().await?;
    
            println!("New client: {}", addr);
        }

        
    }
}