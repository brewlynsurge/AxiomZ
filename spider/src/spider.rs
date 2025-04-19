#![allow(unused)]

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use colored::Colorize;
use serde::{Serialize, Deserialize};
use serde_json;

use crate::url_frontier::UrlForntier;
use super::crawler::Crawler;
use super::utils::Console;

/*
Spider
*/
pub struct Spider {
    state_machine: StateMachine
}

impl Spider {
    pub fn new(data_path: &str) -> Self {
        Self::safe_check_data_path(data_path);
        let state_machine = StateMachine::load(&format!("{}/state.dat", data_path))
            .expect("Failed to load machine state in spider");
        
        Self {
            state_machine
        }
    }

    pub fn start(&mut self) {
        Crawler::crawl_site("");
    }

    fn safe_check_data_path(path: &str) {
        if !std::fs::exists(path).unwrap() {
            std::fs::create_dir(path);
        }
    }
}


/*
StateMachine
*/
#[derive(Serialize, Deserialize, Debug)]
pub struct StateMachine {
    pub urls: HashMap<String, VecDeque<String>>,
    pub total_urls: usize
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            urls: HashMap::new(),
            total_urls: 0
        }
    }

    pub fn load(file_path: &str) -> Result<Self, std::io::Error> {
        Console::info("Creating state...", Some("state_machine"));

        if std::fs::exists(file_path)? {
            println!("  -> Loading state");

            match Self::load_state(file_path) {
                Ok(s) => {
                    println!("  -> done.\n");
                    return Ok(s);
                }
                Err(e) => {
                    println!("  {}{}", "-> Error in loading state: ".red(), e.to_string().red());
                    println!("  -> Recreating new state")
                }
            }
        } else {
            println!("  -> Creating new state") 
        }

        let state = Self::new();
        Self::save_state(&state, file_path);
        println!("  -> done.\n");
        
        Ok(state)
    }

    pub fn save_state(state: &Self, file_path: &str) -> Result<(), std::io::Error> {
        let json_string = serde_json::to_string(&state)?;

        let mut file = std::fs::File::create(file_path)?;
        file.write_all(json_string.as_bytes())?;
        
        Ok(())
    }

    pub fn load_state(file_path: &str) -> Result<Self, std::io::Error> {
        let file = std::fs::File::open(file_path)?;
        let state: Self = serde_json::from_reader(file)?;

        Ok(state)
    }
}