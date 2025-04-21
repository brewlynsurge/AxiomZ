pub mod utils;
pub mod url_frontier;
pub mod crawler;
pub mod spider;


#[tokio::main]
async fn main() {
    let spider_info = utils::Info::new("Spider");
    spider_info.display_head();

    let mut spider_crawler = spider::Spider::new("data");
    spider_crawler.start("https://en.wikipedia.org/wiki/Wikipedia").await;
}
