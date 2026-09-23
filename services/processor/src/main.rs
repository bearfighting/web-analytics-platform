use std::time::Duration;

use chrono::NaiveDate;
use clap::Parser;
use processor::{Processor, WOOTHEE_VERSION};
use tracing::{error, info};

#[derive(Debug, Parser)]
#[command(name = "processor", version, about = "Process Page View raw events")]
struct Cli {
    #[arg(long, default_value_t = false)]
    once: bool,
    #[arg(long, conflicts_with_all = ["rebuild", "backfill", "reparse", "once"], requires = "site_id")]
    rebuild_custom_events: bool,
    #[arg(long, env = "PROCESSOR_POLL_INTERVAL_MS", default_value_t = 1_000)]
    poll_interval_ms: u64,
    #[arg(
        long,
        conflicts_with_all = ["backfill", "reparse", "once"],
        help = "Rebuild a complete site generation; --from/--to record the rebuild scope"
    )]
    rebuild: bool,
    #[arg(long, conflicts_with_all = ["rebuild", "reparse", "once"])]
    backfill: bool,
    #[arg(long, conflicts_with_all = ["rebuild", "backfill", "once"])]
    reparse: bool,
    #[arg(long)]
    site_id: Option<String>,
    #[arg(long, value_parser = parse_date, requires = "site_id")]
    from: Option<NaiveDate>,
    #[arg(long, value_parser = parse_date, requires = "site_id")]
    to: Option<NaiveDate>,
    #[arg(long, requires = "reparse")]
    parser_version: Option<String>,
    #[arg(long, requires = "site_id")]
    dry_run: bool,
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

    if cli.rebuild_custom_events {
        let site_id = cli.site_id.as_deref().expect("clap requires --site-id");
        let count = processor.rebuild_custom_event_facts(site_id).await?;
        info!(site_id, facts = count, "custom event facts rebuilt");
        return Ok(());
    }

    if cli.rebuild || cli.backfill || cli.reparse {
        let site_id = cli
            .site_id
            .ok_or_else(|| anyhow::anyhow!("--site-id is required for rebuild commands"))?;
        let from = cli.from.unwrap_or(NaiveDate::MIN);
        let to = cli.to.unwrap_or(NaiveDate::MAX);
        let parser_version = cli.parser_version.as_deref().unwrap_or(WOOTHEE_VERSION);
        let reason = if cli.reparse {
            "reparse"
        } else if cli.backfill {
            "backfill"
        } else {
            "initial"
        };
        let summary = processor
            .rebuild_site(&site_id, from, to, reason, parser_version, cli.dry_run)
            .await?;
        info!(
            site_id,
            events = summary.events,
            visitors = summary.visitors,
            dry_run = cli.dry_run,
            "phase 6 rebuild complete"
        );
        return Ok(());
    }

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
                if processor.process_rebuild_queue_once().await? {
                    continue;
                }
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

fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|error| format!("invalid date {value}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use chrono::NaiveDate;
    use clap::Parser;

    #[test]
    fn once_mode_is_explicit_and_poll_interval_has_default() {
        let cli = Cli::try_parse_from(["processor", "--once"]).expect("arguments should parse");

        assert!(cli.once);
        assert_eq!(cli.poll_interval_ms, 1_000);
        assert!(!cli.rebuild);
    }

    #[test]
    fn poll_interval_can_be_overridden() {
        let cli = Cli::try_parse_from(["processor", "--poll-interval-ms", "250"])
            .expect("arguments should parse");

        assert!(!cli.once);
        assert_eq!(cli.poll_interval_ms, 250);
    }

    #[test]
    fn rebuild_arguments_parse() {
        let cli = Cli::try_parse_from([
            "processor",
            "--rebuild",
            "--site-id",
            "site_example",
            "--from",
            "2026-09-01",
            "--to",
            "2026-09-18",
            "--dry-run",
        ])
        .expect("rebuild arguments should parse");
        assert!(cli.rebuild);
        assert_eq!(cli.from, Some(NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()));
        assert!(cli.dry_run);
    }
}
