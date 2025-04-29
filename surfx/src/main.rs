pub mod engine;
pub mod website;

// Main function to start the server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let search_engine = engine::SearchEngine::new("data").await?;

    let webserver = website::WebServer::new(search_engine);
    webserver.start().await
}