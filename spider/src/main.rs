pub mod info;
pub mod logs;
pub mod spider;

#[tokio::main]
async fn main() {
    let spider_info = info::Info::new("Spider");
    spider_info.display_head();

    let spider_crawler = spider::Crawler::new();
    spider_crawler.start_crawling().await;
}
