use serde::Deserialize;
use crate::register_configuration;
use crate::config::core::{ConfigLoader, ConfigEntity, Configurations};

// ------------------------------------------------------------
// To register new configuration:
//     - Create a TopLevelConfig Struct
//     - Create required Structs and impl
//     - register_configuration
//     - in 'core.rs', ConfigLoader::new add the field 
// ------------------------------------------------------------

// -------------------- DATABASE CONFIG -----------------------
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
    pub fn load(config_loader: &ConfigLoader) -> DatabaseConfig {
        let toplevel_config = config_loader.load::<DatabaseTopLevelConfig>("DATABASE");
        {
            let (field_safe, field_name) = Self::check_field_saftey(&toplevel_config);
            if !field_safe { ConfigLoader::report_field_safety(&field_name, Configurations::DATABASE.filepath);}
        }
        return toplevel_config.database;
    }

    fn check_field_saftey(toplevel_config: &DatabaseTopLevelConfig) -> (bool, String) {
        if toplevel_config.database.name.is_empty() { return (false, "name".to_string()) }
        else if  toplevel_config.database.password.is_empty() { return (false, "password".to_string()) }
        else if  toplevel_config.database.username.is_empty() { return (false, "username".to_string()) }
        else if  toplevel_config.database.host.is_empty() { return (false, "host".to_string()) }
        else { return (true, "".to_string()) }
    }
}

register_configuration!(DATABASE: "config/database.toml" => r#"
[database]
name = ""
password = ""
username = ""
host = "127.0.0.1"
port = 5001
"#);

// ----------------------- SPIDER CONFIG ----------------------
#[derive(Debug, Deserialize)]
struct SpiderTopLevelConfig {
    spider: SpiderConfig
}

#[derive(Debug, Deserialize)]
pub struct SpiderConfig {
    pub host: String,
    pub port: u16
}

impl SpiderConfig {
    pub fn load(config_loader: &ConfigLoader) -> SpiderConfig {
        let toplevel_config = config_loader.load::<SpiderTopLevelConfig>("SPIDER");
        {
            let (field_safe, field_name) = Self::check_field_saftey(&toplevel_config);
            if !field_safe { ConfigLoader::report_field_safety(&field_name, Configurations::SPIDER.filepath);}
        }
        return toplevel_config.spider;
    }

    fn check_field_saftey(toplevel_config: &SpiderTopLevelConfig) -> (bool, String) {
        if  toplevel_config.spider.host.is_empty() { return (false, "host".to_string()) }
        else { return (true, "".to_string()) }
    }
}

register_configuration!(SPIDER: "config/spider.toml" => r#"
[spider]
host = "127.0.0.1"
port = 5002
"#);
