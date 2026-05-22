use tokio::net::TcpStream;
use shared;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut socket = TcpStream::connect("127.0.0.1:5002").await?;
    
    println!("Connected to server");
    shared::socket::send_data(&mut socket, &"Hellow world".to_string()).await?;
    

    Ok(())
}