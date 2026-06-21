pub mod crawler;
pub mod proxy_rotator;
pub mod axiomz_scraper;
pub mod crawler_animator;

use crossterm::style::Stylize;

// ----------------- MAIN ----------------------
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!(
        "{} [Version {}]",
        "Crawler".cyan().bold(),
        env!("CARGO_PKG_VERSION").italic()
    );
    println!(
        "(c) {}. All rights are reserved.\n",
        "AxiomZ".cyan().italic()
    );

    // Crawler
    let mut crawler = crawler::Crawler::build().await?;
    crawler.start().await?;

    Ok(())
}
