use tokio::net::TcpListener;
use std::sync::Arc;
use tokio::sync::Mutex;
use shared;
use super::handler;
use crate::url_frontier::core::UrlFrontier;
use spider_shared::database::AxiomZDatabase;

pub async fn handle_spider_server(url_frontier: Arc<Mutex<UrlFrontier>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let spider_server = Server::start().await?;
    println!("Starting Server...");

    spider_server.handle_tcp_connections(url_frontier).await?;
    Ok(())
}


// ------------- SPIDER SERVER ---------------------
pub struct Server {
    listener: TcpListener,
    pub host: String,
    pub port: u16,
    database_config: shared::config::DatabaseConfig,
    pub database: Arc<Mutex<AxiomZDatabase>>
}

impl Server {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_loader = shared::config::ConfigLoader::new()
            .resolve("DATABASE")
            .resolve("SPIDER")
            .execute_resolves();

        let database_config = shared::config::DatabaseConfig::load(&config_loader);
        let spider_config = shared::config::SpiderConfig::load(&config_loader);
        
        // Starting server
        let tcp_listener = TcpListener::bind(format!("{}:{}", spider_config.host, spider_config.port)).await?;

        // Connecting to database
        let axiomz_database = AxiomZDatabase::connect(&database_config).await?;
        axiomz_database.run_migrations().await?;

        Ok(Self {
            listener: tcp_listener,
            host: spider_config.host,
            port: spider_config.port,
            database_config: database_config,
            database: Arc::new(Mutex::new(axiomz_database))
        })
    }

    pub async fn handle_tcp_connections(&self, url_frontier: Arc<Mutex<UrlFrontier>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let (socket, _) = self.listener.accept().await?;
            let database_config = self.database_config.clone();

            let frontier = url_frontier.clone();
            tokio::spawn(async move {
                match handler::ClientHandler::handle_client(socket, database_config, frontier).await {
                    Ok(_) => {},
                    Err(_) => {
                        // TODO: if need, handle error
                    } 
                };
            });
        }
    }
}