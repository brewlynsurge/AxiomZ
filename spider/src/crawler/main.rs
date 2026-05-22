fn main() {
    let config_loader = shared::config::ConfigLoader::new()
        .resolve("DATABASE")
        .resolve("SPIDER")
        .execute_resolves();

    let spider_config = shared::config::SpiderConfig::load(&config_loader);
    println!("{:?}", spider_config)
}