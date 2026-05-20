use std::collections::HashMap;
use crossterm::style::Stylize;
use toml;

// -------------------- CONFIGURATION -------------------------
pub struct ConfigEntity {
    pub name: &'static str,
    pub filepath: &'static str,
    pub toml_structure: &'static str
}
pub struct Configurations;

#[macro_export]
macro_rules! register_configuration {
    ($name: ident : $path: literal => $toml_structure: literal) => {
        impl Configurations {
            pub const $name:ConfigEntity = ConfigEntity {
                name: stringify!($name),
                filepath: $path,
                toml_structure: $toml_structure
            };
        }
    };
}

// -------------------- CONFIG LOADER -------------------------
pub struct ConfigLoader {
    pub configurations: HashMap<String, ConfigEntity>,
    resolves: Vec<String>
}

impl ConfigLoader {
    pub fn new() -> Self {
        let mut config_loader = Self {
            configurations: HashMap::new(),
            resolves: Vec::new()
        };

        // Loading registrations into the runtime enviornment
        config_loader.configurations.insert(String::from("DATABASE"), Configurations::DATABASE);
        config_loader.configurations.insert(String::from("SPIDER"), Configurations::SPIDER);
        
        return config_loader;
    }

    pub fn load<T>(&self, config_name: &str) -> Result<T, Box<dyn std::error::Error + Send + Sync>> 
    where T:serde::de::DeserializeOwned
    {
        let config_path = self.configurations.get(config_name).unwrap().filepath;
        let file_content = std::fs::read_to_string(config_path)?;
        let config: T = toml::from_str(&file_content)?;
        Ok(config)
    }

    pub fn report_field_safety(field_name: &str, config_path: &str) {
        println!("{} Missing Configuration Field", "ConfigError:".red());
        println!("   -> The field {} should contain a valid value in '{}'", field_name.red(), config_path.red());
        std::process::exit(1)
    }

    pub fn resolve(mut self, config_name: &str) -> Self {
        self.resolves.push(config_name.to_string());
        self
    }

    pub fn execute_resolves(mut self) -> Self {
        let mut resolved_configs:Vec<String> = Vec::new();
        for config_name in self.resolves.iter() {
            let config_path = std::path::Path::new(self.configurations.get(config_name).unwrap().filepath);
            if !config_path.exists() {
                let _ = std::fs::write(config_path, self.configurations.get(config_name).unwrap().toml_structure);
                resolved_configs.push(config_name.to_string());
            }
        }

        if !resolved_configs.is_empty() { println!("{} Missing Configurations", "ConfigError:".red()) }
        for config_name in resolved_configs.iter() {
            println!("   -> Created new {} config toml", config_name.to_lowercase());
        }
        
        if !resolved_configs.is_empty() {
            println!("   -> Please fill the config");
            std::process::exit(1);
        }
        self.resolves.clear();
        
        self
    }
}