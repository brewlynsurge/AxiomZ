use shared;

fn main() {
    let config_loader = shared::config::ConfigLoader::new()
        .resolve("DATABASE")
        .resolve("SPIDER")
        .execute_resolves();

    let database_config = shared::config::DatabaseConfig::load(&config_loader);
    println!("{:?}", database_config)
}