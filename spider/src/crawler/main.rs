use shared;

fn main() {
    let database = shared::DatabaseConfig::load();
    println!("{:?}", database)
}