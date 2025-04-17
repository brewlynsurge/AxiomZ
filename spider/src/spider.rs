#![allow(unused)]


use std::io::Write;
use super::logs::Console;

pub struct Crawler;

impl Crawler {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn start_crawling(&self) {
        print!("Starting Spider Crawler:");
        std::thread::sleep(std::time::Duration::from_millis(500));
        
        
    }
}