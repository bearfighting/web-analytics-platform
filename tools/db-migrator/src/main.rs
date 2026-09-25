use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let first_argument = arguments.next();
    if first_argument.as_deref() == Some("--purge-expired-configuration-audit") {
        anyhow::ensure!(
            arguments.next().is_none(),
            "unexpected arguments after --purge-expired-configuration-audit"
        );
        let database_url = std::env::var("DATABASE_URL")
            .context("DATABASE_URL must be configured for database maintenance")?;
        let deleted = db_migrator::purge_expired_configuration_audit(&database_url).await?;
        println!("purged_expired_configuration_audit={deleted}");
        return Ok(());
    }

    if first_argument.as_deref() == Some("--target-version") {
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
        first_argument.is_none() && arguments.next().is_none(),
        "usage: db-migrator [--target-version VERSION | --purge-expired-configuration-audit]"
    );
    db_migrator::run_from_environment().await
}
