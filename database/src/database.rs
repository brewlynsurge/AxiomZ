use sqlx::{Pool, SqlitePool, Sqlite};

pub struct SurfXDatabase {
    database_url: String,
    pool: Option<Pool<Sqlite>>
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

        Ok(())
    }

    pub async fn add_webpage(&self, title: &str, url: &str, description: Option<String>) -> Result<i32, std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
        
        let id: Option<(i32,)> = sqlx::query_as("
            INSERT INTO webpages (url, title, description)
            VALUES (?, ?, ?)
            ON CONFLICT(url) DO NOTHING
            RETURNING webpage_id;
        ")
        .bind(url)
        .bind(title)
        .bind(description.unwrap_or_default())
        .fetch_optional(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        if let Some((id,)) = id {
            return Ok(id);
        } else {
            // If conflict happened, fetch the existing one
            let id: (i32,) = sqlx::query_as("SELECT webpage_id FROM webpages WHERE url = ?;")
                .bind(url)
                .fetch_one(pool)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
            return Ok(id.0);
        }
    }

    pub async fn add_word(&self, word: &str) -> Result<i32, std::io::Error> {
        let pool = self.pool.as_ref().unwrap();
        
        // Insert or ignore the word and get its ID
        let word_id: Option<(i32,)> = sqlx::query_as(r#"
            INSERT INTO words (word)
            VALUES (?)
            ON CONFLICT (word) DO NOTHING
            RETURNING word_id;
        "#)
        .bind(word)
        .fetch_optional(pool)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        
        let word_id = match word_id {
            Some((id,)) => id,
            None => {
                // Word already exists, so we select the ID
                let row: (i32,) = sqlx::query_as("SELECT word_id FROM words WHERE word = ?")
                    .bind(word)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
                row.0
            }
        };

        Ok(word_id)
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
}