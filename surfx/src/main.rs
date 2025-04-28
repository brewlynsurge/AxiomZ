pub mod surfx;

#[tokio::main]
async fn main() {
    let search_engine =  surfx::SearchEngine::new("data").await.unwrap();
    
    let results = search_engine.search("india").await;
    for i in results {
        println!("{i}");
    }
}
