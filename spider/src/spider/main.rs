mod server;

use crossterm::style::Stylize;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("{} [Version {}]", "Spider".cyan().bold(), env!("CARGO_PKG_VERSION").italic());
    println!("(c) {}. All rights are reserved.\n", "AxiomZ".cyan().italic());

    let mut axiomz_server = server::AxiomZServer::new();
    axiomz_server.start().await?;
    Ok(())
}