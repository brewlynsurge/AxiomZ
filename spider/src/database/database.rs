use std::collections::{HashMap, HashSet};
use sqlx::{Executor, Postgres, Transaction};
use shared::config::DatabaseConfig;

// ----------------- AXIOMZ DATABASE ----------------------
pub struct AxiomZDatabase {
    pub pool: sqlx::postgres::PgPool
}

impl AxiomZDatabase {
    pub async fn connect(database_config: &DatabaseConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("postgres://{}:{}@{}:{}/{}", database_config.username, database_config.password, database_config.host, database_config.port, database_config.name);
        let pool = sqlx::postgres::PgPool::connect(&url).await?;

        Ok(Self {
            pool: pool
        })
    }

    pub async fn run_migrations(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
    
    // NOTE: Only use while developing application
    pub async fn reset_database(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.pool.execute("DROP SCHEMA public CASCADE").await?;
        self.pool.execute("CREATE SCHEMA public").await?;
        Ok(())
    }
}

// ----------------- DATABASE DOCUMENTS TABLE ----------------------
pub struct DatabaseDocumentsTable;
impl DatabaseDocumentsTable {
    pub async fn insert(tx: &mut Transaction<'_, Postgres>, page_url: &str, url_hash: i64, title: &String, description: &String, document_len: usize) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
        let document_id: Option<i64> = sqlx::query_scalar(r#"
            INSERT INTO documents (url, url_hash, title, description, document_len)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (url_hash) DO NOTHING
            RETURNING id
        "#)
        .bind(page_url)
        .bind(url_hash)
        .bind(title)
        .bind(description)
        .bind(document_len as i32)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(document_id)
    }
}

// ----------------- DATABASE TERMS TABLE ----------------------
pub struct DatabaseTermsTable;
impl DatabaseTermsTable {
    pub async fn insert_terms(tx: &mut Transaction<'_, Postgres>, words: &HashMap<String, usize>) -> Result<Vec<(i64, String)>, Box<dyn std::error::Error + Send + Sync>> {
        let mut query = sqlx::QueryBuilder::new(
            "INSERT INTO terms (term, document_freq)"
        );
        
        query.push_values(words.keys(), |mut row, term| {
            row.push_bind(term)
               .push_bind(1_i32);
        });

        query.push(r#"
            ON CONFLICT (term)
            DO UPDATE
            SET document_freq = terms.document_freq + 1
            RETURNING id, term
        "#,);

        let term_rows: Vec<(i64, String)> = query
            .build_query_as()
            .fetch_all(&mut **tx)
            .await?;
        
        Ok(term_rows)
    }
}

// ----------------- DATABASE POSTINGS TABLE ----------------------
pub struct DatabasePostingsTable;
impl DatabasePostingsTable {
    pub async fn insert_postings(tx: &mut Transaction<'_, Postgres>, words: &HashMap<String, usize>, term_rows: Vec<(i64, String)>, document_id: i64, document_len: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut query = sqlx::QueryBuilder::new(
            "INSERT INTO postings (term_id, doc_id, tf)"
        );

        query.push_values(term_rows.iter(), |mut row, (term_id, term)| {
            row.push_bind(*term_id)
                .push_bind(document_id)
                .push_bind(Self::calculate_tf(words, term, document_len));
        });

        query.build()
            .execute(&mut **tx)
            .await?;
        
        Ok(())
    }

    fn calculate_tf(words: &HashMap<String, usize>, term: &str, document_len: usize) -> f32 {
        if document_len == 0 {return 0.0}
        
        let term_count = words[term];
        term_count as f32 / document_len as f32
    }
}

// ----------------- DATABASE FRONTIER URS TABLE ----------------------
pub struct DatabaseFrontierUrlsTable;
impl DatabaseFrontierUrlsTable {
    pub async fn insert_urls(tx: &mut Transaction<'_, Postgres>, urls: &HashSet<String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if urls.is_empty() { return Ok(()) }

        let mut query = sqlx::QueryBuilder::new(
            "INSERT INTO frontier_urls (url)"
        );

        query.push_values(urls.iter(), |mut row, url| {
            row.push_bind(url);
        });
        query.push(" ON CONFLICT (url) DO NOTHING");

        query.build()
            .execute(&mut **tx)
            .await?;
        Ok(())
    }
}