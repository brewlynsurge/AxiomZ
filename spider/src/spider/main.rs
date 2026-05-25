pub mod url_frontier;
pub mod server;
pub mod terminal_ui;

use std::sync::Arc;
use tokio::sync::Mutex;
use url_frontier::core::UrlFrontier;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url_frontier: Arc<Mutex<UrlFrontier>> = Arc::new(Mutex::new(UrlFrontier::new()));

    // Spawns spider server
    tokio::spawn(async move {
        server::core::handle_spider_server().await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // Handle terminal output
    terminal_ui::handle_terminal_renderer().await?;
    
    Ok(())
}