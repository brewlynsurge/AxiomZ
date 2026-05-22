use std::net::SocketAddr;
use tokio::net::TcpStream;
use shared;

// ------------- CLIENT HANDLER ---------------------
pub struct ClientHandler;
impl ClientHandler {
    pub async fn handle_client(mut socket: TcpStream, addr:SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>>{
        println!("Client connected: {}", addr);

        loop {
            let cleint_msg: String = shared::socket::receive_data(&mut socket).await?;
            println!("{}", cleint_msg)
        }
    }
}