use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    db_migrator::run_from_environment().await
}
