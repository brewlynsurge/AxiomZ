pub mod server;
pub mod terminal_ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Spawns spider server
    tokio::spawn(async move {
        server::core::handle_spider_server().await?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // Handle terminal output
    terminal_ui::handle_terminal_renderer().await?;
    
    Ok(())
}