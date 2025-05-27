#![allow(unused)]

use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::{HashSet,HashMap};
use regex::Regex;
use super::utils;
use database::database::SurfXDatabase;

pub struct Indexer {
    title: String,
    url: String,
    description: Option<String>
}

impl Indexer {
    pub fn new(title: &str, url: &str, description: &str) -> Self {
        let description = {
            if description.is_empty() { None }
            else { Some(description.to_string()) }
        };

        Self {
            title: String::from(title),
            url: String::from(url),
            description: description
        }
    }

    pub async fn parse(&self, page_texts: HashSet<String>, surfx_database: Arc<Mutex<SurfXDatabase>>) -> Result<(), std::io::Error> {
        let db = surfx_database.lock().await;
        // TODO: Fix some letters of words are missing sometimes    

        let stop_words: HashSet<String> = utils::STOP_WORDS.iter()
            .map(|s: &&str| s.to_string()).collect();
        let total_webpages = db.count_total_webpages().await? + 1;

        let regex_english = Regex::new(r"^[a-zA-Z0-9_]+$")
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        let english_stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);

        let mut words_hash = HashMap::new();
        let mut total_words = 0;
        for text in page_texts {
            for word in text.split_whitespace() {
                // Check if it is english or else skip
                if !regex_english.is_match(&word) {continue;}

                let word = word
                    .to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect::<String>();

                let stemmerized_word = english_stemmer.stem(&word).to_string();

                if stop_words.contains(&stemmerized_word) {
                    continue;
                }

                let frequency = *words_hash.get(&stemmerized_word).unwrap_or(&0);
                words_hash.insert(stemmerized_word, frequency+1);
                total_words += 1;
            }
        }

        let webpage_id = db.add_webpage(&self.title, &self.url, self.description.clone()).await?;
        for (word, freq) in words_hash {
            let word_id = db.add_word(&word).await?;
            
            let tf = freq as f64 / total_words as f64;
            let idf = {
                let word_occurrence = db.count_word_occurrences(word_id).await? + 1;
                (total_webpages as f64 / word_occurrence as f64).log10()
            };
            let tf_idf = tf * idf;
            db.add_link(word_id, webpage_id, tf_idf).await?;
        }

        Ok(())
    }
}
