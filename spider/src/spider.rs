#![allow(unused)]

use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::process::exit;
use tokio::sync::Mutex;
use std::sync::Arc;
use colored::Colorize;
use serde::{Serialize, Deserialize};
use serde_json;

use super::url_frontier::UrlFrontier;
use super::crawler::Crawler;
use super::utils::Console;
use super::indexer::Indexer;
use database::database::SurfXDatabase;

/*
Spider
*/
pub struct Spider {
    data_path: String,
    database: Arc<Mutex<SurfXDatabase>>,
    url_frontier: UrlFrontier,
}

impl Spider {
    pub fn new(data_path: &str) -> Self {
        Self::safe_check_data_path(data_path);
        
        let surfx_database = SurfXDatabase::new(data_path);
        let url_frontier = UrlFrontier::new("https://en.wikipedia.org/wiki/Main_Page");

        Self {
            data_path: data_path.to_string(),
            database: Arc::new(Mutex::new(surfx_database)),
            url_frontier: url_frontier,
        }
    }

    pub async fn start(&mut self, start_url: &str) {
        // Connect to the database
        match self.database.lock().await.connect().await {
            Ok(_) => {},
            Err(e) => {
                Console::error(&format!("Failed to connect to the database: {e}"), Some("spider"));
                return ;
            }
        }

        // Load the state from the database
        self.url_frontier.load_state(self.database.clone()).await;

        let mut current_url = self.url_frontier.get_init_url(self.database.clone()).await;
        
        loop {
            Console::info(&format!("Crawling {current_url}"), Some("spider"));
            let page_container = Crawler::crawl_site(&current_url).await;

            if page_container.is_ok() {
                let (page_title, page_description, page_texts, page_links) = page_container.unwrap();
                use std::time::Instant;
                let start = Instant::now();
                self.url_frontier.extend(page_links, self.database.clone()).await;

                let duration = start.elapsed();
                let page_indexer = Indexer::new(&page_title, &current_url, &page_description);
                page_indexer.parse(page_texts, self.database.clone()).await;
            } else {
                Console::error(&format!("Page crawl error: {}", page_container.err().unwrap()), Some("spider"));
            }
            
            current_url = loop {
                if let Some(url) = self.url_frontier.get_url(self.database.clone()).await {
                    break url;
                }

                // Optionally, sleep for a short duration before retrying
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

        }
        

        /* 
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
        */
    }

    /*
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
    */

    fn safe_check_data_path(path: &str) {
        if !std::fs::exists(path).unwrap() {
            std::fs::create_dir(path);
        }
    }

    
}