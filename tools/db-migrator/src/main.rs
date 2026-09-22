use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some("--target-version") {
        let target = arguments
            .next()
            .context("--target-version requires a migration version")?
            .parse::<i64>()
            .context("--target-version must be an integer")?;
        anyhow::ensure!(
            arguments.next().is_none(),
            "unexpected arguments after --target-version"
        );
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL must be configured for database migrations")?;
        return db_migrator::run_until(&database_url, target).await;
    }

    anyhow::ensure!(
        arguments.next().is_none(),
        "usage: db-migrator [--target-version VERSION]"
    );
    db_migrator::run_from_environment().await
}
