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
}