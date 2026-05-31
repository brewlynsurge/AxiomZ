use tokio::net::TcpStream;
use std::sync::Arc;
use tokio::sync::Mutex;
use shared;
use crate::url_frontier::core::UrlFrontier;

// ------------- CLIENT HANDLER ---------------------
pub struct ClientHandler;
impl ClientHandler {
    pub async fn handle_client(mut socket: TcpStream, database_config: shared::config::DatabaseConfig, url_frontier: Arc<Mutex<UrlFrontier>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>>{
        // Send database configuration to the crawler
        shared::socket::send_data(&mut socket, &database_config).await?;

        match shared::socket::receive_data::<String>(&mut socket).await {
            Ok(command) => {
                Self::handle_command(&command, &mut socket, url_frontier).await.unwrap();
            },
            Err(_) => {
                //TODO
            }
        };
        
        Ok(())
    }

    async fn handle_command(command: &str, stream: &mut TcpStream, url_frontier: Arc<Mutex<UrlFrontier>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match command {
            "GET_URL" => {
                let frontier = url_frontier.lock().await;
                shared::socket::send_data(stream, &frontier.get_url()).await.unwrap();
            }
            _ => {
                // TODO
            }
        };

        Ok(())
    }
}