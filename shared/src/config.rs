use std::error::Error;
use serde::Deserialize;
use toml;

// ------------- CONFIG PATHS ---------------------
const DATABSE_CONFIG_PATH: &str = "config/database.toml";
// ------------------------------------------------


// ------------- DATABASE CONFIG  ---------------------
#[derive(Debug, Deserialize)]
struct DatabaseTopLevelConfig {
    database: DatabaseConfig
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub name: String,
    pub password: String,
    pub username: String,
    pub host: String,
    pub port: u16,
}

impl DatabaseConfig {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let file_content = std::fs::read_to_string(DATABSE_CONFIG_PATH)?;
        let config:DatabaseTopLevelConfig = toml::from_str(&file_content)?;

        return Ok(config.database)
    }
}