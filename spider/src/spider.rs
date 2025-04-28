#![allow(unused)]

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::process::exit;
use colored::Colorize;
use serde::{Serialize, Deserialize};
use serde_json;

use crate::url_frontier::UrlForntier;
use super::crawler::Crawler;
use super::utils::Console;
use super::indexer::Indexer;
use database::database::SurfXDatabase;

/*
Spider
*/
pub struct Spider {
    data_path: String,
    state_machine: StateMachine,
    database: SurfXDatabase
}

impl Spider {
    pub fn new(data_path: &str) -> Self {
        Self::safe_check_data_path(data_path);
        let state_machine = StateMachine::load(&format!("{}/state.dat", data_path))
            .expect("Failed to load machine state in spider");
        
        let surfx_database = SurfXDatabase::new(data_path);
        Self {
            data_path: data_path.to_string(),
            state_machine,
            database: surfx_database
        }
    }

    pub async fn start(&mut self, start_url: &str) {
        // Connect to the database
        match self.database.connect().await {
            Ok(_) => {},
            Err(e) => {
                Console::error(&format!("Failed to connect to the database: {e}"), Some("spider"));
                return ;
            }
        }
        
        // Start crawling
        let mut current_url = {
            if self.state_machine.urls.is_empty() {start_url.to_string()}
            else {Self::get_next_url(&mut self.state_machine)}

        };

        loop {
            Console::info(&format!("Crawling {current_url}"), Some("spider"));

            let page_container = Crawler::crawl_site(&current_url).await;
            if page_container.is_ok() {
                let (page_title, page_description, page_texts, page_links) = page_container.unwrap();

                let page_indexer = Indexer::new(&page_title, &current_url, &page_description);
                page_indexer.parse(page_texts, &self.database).await;
                UrlForntier::extend(page_links, &mut self.state_machine);
                StateMachine::save_state(&self.state_machine, &format!("{}/state.dat", &self.data_path));
            } else {
                Console::error(&format!("Page crawl error: {}", page_container.err().unwrap()), Some("spider"));
            }

            current_url = Self::get_next_url(&mut self.state_machine);
        }
    
    }

    fn get_next_url(state_machine: &mut StateMachine) -> String {
        loop {
            let new_url = UrlForntier::get_url(state_machine);
            if new_url.is_none() {
                Console::warn("Failed to retrive url from UrlForntier", Some("spider"));
                continue;
            }
            break new_url.unwrap();

        }
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