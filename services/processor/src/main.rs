use std::time::Duration;

use clap::Parser;
use processor::Processor;
use tracing::{error, info};

#[derive(Debug, Parser)]
#[command(name = "processor", version, about = "Process Page View raw events")]
struct Cli {
    #[arg(long, default_value_t = false)]
    once: bool,
    #[arg(long, env = "PROCESSOR_POLL_INTERVAL_MS", default_value_t = 1_000)]
    poll_interval_ms: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("DATABASE_URL must be configured"))?;
    let processor = Processor::connect(&database_url).await?;

    if cli.once {
        let processed = processor.process_all_once().await?;
        info!(processed, "processor backlog complete");
        return Ok(());
    }

    let interval = Duration::from_millis(cli.poll_interval_ms);
    loop {
        match processor.process_one().await {
            Ok(true) => {}
            Ok(false) => {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = tokio::signal::ctrl_c() => {
                        info!("processor stopped");
                        return Ok(());
                    }
                }
            }
            Err(error) => {
                error!(%error, "processor transaction failed; will retry");
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {}
                    _ = tokio::signal::ctrl_c() => {
                        info!("processor stopped");
                        return Ok(());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::Parser;

    #[test]
    fn once_mode_is_explicit_and_poll_interval_has_default() {
        let cli = Cli::try_parse_from(["processor", "--once"]).expect("arguments should parse");

        assert!(cli.once);
        assert_eq!(cli.poll_interval_ms, 1_000);
    }

    #[test]
    fn poll_interval_can_be_overridden() {
        let cli = Cli::try_parse_from(["processor", "--poll-interval-ms", "250"])
            .expect("arguments should parse");

        assert!(!cli.once);
        assert_eq!(cli.poll_interval_ms, 250);
    }
}
