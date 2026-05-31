use tokio::net::TcpStream;
use shared;

// ----------------- SCRAPER ----------------------
pub struct Scraper;

impl Scraper {
    pub async fn scrap_page(stream: &mut TcpStream) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = Self::get_url(stream).await?;
        println!("{url}");

        Ok(())
    }

    async fn get_url(stream: &mut TcpStream) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        shared::socket::send_data::<String>(stream, &String::from("GET_URL")).await?;
        let url = shared::socket::receive_data::<String>(stream).await?;

        
        return Ok(url);
    }

    
    
}
