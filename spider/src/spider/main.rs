pub mod url_frontier;
pub mod server;
pub mod terminal_ui;

use std::sync::Arc;
use tokio::sync::Mutex;
use crossterm::style::Stylize;
use url_frontier::core::UrlFrontier;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{} [Version {}]", "Spider".cyan().bold(), env!("CARGO_PKG_VERSION").italic());
    println!("(c) {}. All rights are reserved.\n", "AxiomZ".cyan().italic());

    let url_frontier: Arc<Mutex<UrlFrontier>> = Arc::new(Mutex::new(UrlFrontier::new()));

    // Spawns spider server
    let frontier_clone = url_frontier.clone();
    tokio::spawn(async move {
        server::core::handle_spider_server(frontier_clone).await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // Handle terminal output
    terminal_ui::handle_terminal_renderer().await?;
    
    Ok(())
}