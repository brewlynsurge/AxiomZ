mod server;
mod terminal;
use terminal::{TerminalHandler, AXIOMZ_TERMINAL, TerminalRenderer};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _alternate_screen_guard = terminal::AlternateScreenGuard::new()?;

    terminal::log!("Helllo");

    terminal::log!(OVERWRITE -> "Hi");

    loop {

    }


    let mut axiomz_server = server::AxiomZServer::new();
    axiomz_server.start().await?;
    Ok(())
}
