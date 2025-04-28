use sqlx::{Pool, Sqlite};
use sqlx::Row;
use database::database::SurfXDatabase;
use std::collections::HashSet;


/* 
Global Variables
*/
pub const STOP_WORDS: [&str; 33]  = [
    "a", "an", "and", "are", "as", "at", "be", "but", "by", "for", "if", "in", "into", "is",
    "it", "no", "not", "of", "on", "or", "such", "that", "the", "their", "then", "there",
    "these", "they", "this", "to", "was", "will", "with",
];

pub struct SearchEngine {
    pub database: SurfXDatabase
}

impl SearchEngine {
    pub async fn new(data_path: &str) -> Result<Self, std::io::Error> {
        let mut surfx_databse = SurfXDatabase::new(data_path);
        surfx_databse.connect().await?;

        Ok(Self {
            database: surfx_databse
        })
    }

    pub async fn search(&self, search_input: &str) -> HashSet<String> {
        let pool: &Pool<Sqlite> = self.database.pool.as_ref().unwrap();
        
        let search_input = Self::filter_search_input(search_input);
        let input_ids = Self::covert_to_word_ids(search_input, pool).await;

        let mut urls_tfidf: Vec<(String, f64)> = Vec::new();
        for word_id in input_ids {
            if let Ok(websites_data) = Self::get_websites_from_word_id(word_id, pool).await {
                urls_tfidf.extend(websites_data.into_iter().map(|web_data| (web_data.0, web_data.3)));
            }
        }

        // Sort by tfidf in descending order
        urls_tfidf.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Extract sorted URLs and tfidf values
        let sorted_search_urls: HashSet<_> = urls_tfidf.iter().map(|(url, _)| url.clone()).collect();

        return sorted_search_urls;

    }

    async fn get_websites_from_word_id(word_id: i32, pool: &Pool<Sqlite>) -> Result<Vec<(String, String, Option<String>, f64)>, std::io::Error>{
        let query  = r#"
            SELECT webpages.url, webpages.title, webpages.description, word_page_link.tf_idf
            FROM word_page_link
            INNER JOIN webpages ON word_page_link.webpage_id = webpages.webpage_id
            WHERE word_id = ?
        "#;

        let rows = sqlx::query(query)
            .bind(word_id)
            .fetch_all(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;


        let results: Vec<(String, String, Option<String>, f64)> = rows
            .iter()
            .map(|row| {
                (
                    row.get::<String, _>("url"),
                    row.get::<String, _>("title"),
                    row.get::<Option<String>, _>("description"), // Handle nullable description
                    row.get::<f64, _>("tf_idf"),
                )
            })
            .collect();

        return Ok(results);
    }

    async fn covert_to_word_ids(search_input: Vec<String>, pool: &Pool<Sqlite>) -> HashSet<i32> {
        let mut word_ids = HashSet::new();
        for word in search_input {
            let word_id: Result<(i32,), std::io::Error> = sqlx::query_as("SELECT word_id FROM words WHERE word = ?")
                .bind(word)
                .fetch_one(pool)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
            
            if word_id.is_ok() {
                word_ids.insert(word_id.unwrap().0);
            }
        }

        return word_ids;
    }

    pub fn filter_search_input(search_input: &str) -> Vec<String> {
        let input = search_input
            .to_lowercase()
            .split_whitespace()
            .filter(|c| !STOP_WORDS.contains(c))
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        return input;
    }
}