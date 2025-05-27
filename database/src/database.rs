use sqlx::{Pool, SqlitePool, Sqlite};
use sqlx::Row; // Import this to use `.get()` on rows


pub struct SurfXDatabase {
    database_url: String,
    pub pool: Option<Pool<Sqlite>>
}

impl SurfXDatabase {
    pub fn new(data_directory_path: &str) -> Self {
        let database_path = format!("{data_directory_path}/database.db");
        if !std::fs::exists(&database_path).unwrap() {_ = std::fs::File::create(&database_path);}

        let database_url = format!("sqlite://{database_path}");
        Self {
            database_url,
            pool: None
        }
    }

    pub async fn connect(&mut self) -> Result<(), std::io::Error> {
        self.pool = Some(
            SqlitePool::connect(&self.database_url).await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
        );
        self.create_tables().await?;

        Ok(())
    }

    async fn create_tables(&self) -> Result<(), std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
        
        // webpages Table
        sqlx::query("CREATE TABLE IF NOT EXISTS webpages (
            webpage_id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL UNIQUE,
            title TEXT,
            description TEXT DEFAULT NULL
        )").execute(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // words Table
        sqlx::query("CREATE TABLE IF NOT EXISTS words (
            word_id INTEGER PRIMARY KEY AUTOINCREMENT,
            word TEXT NOT NULL UNIQUE
        )").execute(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // word webpage link Table
        sqlx::query("CREATE TABLE IF NOT EXISTS word_page_link (
            webpage_id INTEGER NOT NULL,
            word_id INTEGER NOT NULL,
            tf_idf REAL NOT NULL,

            PRIMARY KEY (webpage_id, word_id),
            FOREIGN KEY (webpage_id) REFERENCES webpages(webpage_id) ON DELETE CASCADE,
            FOREIGN KEY (word_id) REFERENCES words(word_id) ON DELETE CASCADE
        )").execute(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        
        // Visited sites table
        sqlx::query("CREATE TABLE IF NOT EXISTS visited_sites (url TEXT NOT NULL UNIQUE)")
            .execute(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        // url frontier table
        sqlx::query("CREATE TABLE IF NOT EXISTS url_frontier (
            url TEXT NOT NULL UNIQUE,
            priority REAL NOT NULL,
            PRIMARY KEY (url)
        )").execute(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    pub async fn add_webpage(&self, title: &str, url: &str, description: Option<String>) -> Result<i32, std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
    
        // Insert or ignore the webpage
        sqlx::query("
            INSERT INTO webpages (url, title, description)
            VALUES (?, ?, ?)
            ON CONFLICT(url) DO NOTHING;
        ")
        .bind(url)
        .bind(title)
        .bind(description.unwrap_or_default())
        .execute(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
        // Fetch the webpage_id
        let id: (i32,) = sqlx::query_as("SELECT webpage_id FROM webpages WHERE url = ?;")
            .bind(url)
            .fetch_one(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
        Ok(id.0)
    }

    pub async fn add_word(&self, word: &str) -> Result<i32, std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
    
        // Insert or ignore the word
        sqlx::query(r#"
            INSERT INTO words (word)
            VALUES (?)
            ON CONFLICT (word) DO NOTHING;
        "#)
        .bind(word)
        .execute(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
        // Fetch the word_id
        let row: (i32,) = sqlx::query_as("SELECT word_id FROM words WHERE word = ?")
            .bind(word)
            .fetch_one(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
        Ok(row.0)
    }

    pub async fn add_link(&self, word_id: i32, webpage_id: i32, tf_idf: f64) -> Result<(), std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
        
        // Insert or update the word_page_link
        sqlx::query(r#"
            INSERT INTO word_page_link (webpage_id, word_id, tf_idf)
            VALUES (?, ?, ?)
            ON CONFLICT (webpage_id, word_id) DO UPDATE SET tf_idf = excluded.tf_idf;
        "#)
        .bind(webpage_id)
        .bind(word_id)
        .bind(tf_idf)
        .execute(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        Ok(())
    }

    pub async fn count_total_webpages(&self) -> Result<i64, std::io::Error> {
        let pool = self.pool.as_ref().unwrap();

        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM webpages")
            .fetch_one(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(row.0)
    }

    pub async fn count_word_occurrences(&self, word_id: i32) -> Result<i64, std::io::Error> {
        let pool = self.pool.as_ref().unwrap();

        // Count occurrences in word_page_link
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM word_page_link WHERE word_id = ?")
            .bind(word_id)
            .fetch_one(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(row.0)
    }

    pub async fn print_all_webpages(&self) -> Result<(), std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
    
        let rows = sqlx::query(
            r#"
            SELECT webpage_id, url, title, description
            FROM webpages
            ORDER BY webpage_id
            "#
        )
        .fetch_all(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
        println!("--- Webpages Table ---");
        for row in rows {
            let webpage_id: i64 = row.get("webpage_id");
            let url: String = row.get("url");
            let title: Option<String> = row.get("title");
            let description: Option<String> = row.get("description");
    
            println!(
                "ID: {}, URL: {}, Title: {}, Description: {}",
                webpage_id,
                url,
                title.unwrap_or_default(),
                description.unwrap_or_default()
            );
        }
    
        Ok(())
    }

    pub async fn insert_to_url_frontier(&self, url: &str, priority: f64) -> Result<(), std::io::Error> {
        let pool = self.pool.as_ref().unwrap();

        sqlx::query("INSERT INTO url_frontier (url, priority)
                        VALUES (?, ?) ON CONFLICT (url)
                        DO UPDATE SET priority = EXCLUDED.priority;"
                    )
            .bind(url)
            .bind(priority)
            .execute(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }
    
    pub async fn remove_from_url_frontier(&self, url: &str) -> Result<(), std::io::Error> {
        let pool = self.pool.as_ref().unwrap();

        sqlx::query("DELETE FROM url_frontier WHERE url = ?")
            .bind(url)
            .execute(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        Ok(())
    }

    pub async fn get_all_urls_from_url_frontier(&self) -> Result<Vec<(String, f64)>, std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
    
        let rows = sqlx::query_as::<_, (String, f64)>("SELECT url, priority FROM url_frontier")
            .fetch_all(pool)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    
        Ok(rows)
    }
}