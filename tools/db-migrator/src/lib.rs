use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;

pub async fn run_from_environment() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be configured for database migrations")?;
    run(&database_url).await
}

pub async fn run(database_url: &str) -> Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await
        .context("failed to connect to PostgreSQL for database migrations")?;

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .context("database migrations failed")?;

    pool.close().await;
    Ok(())
}
