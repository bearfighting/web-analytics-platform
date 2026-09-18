use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use tracing::info;

use collector::cli::{Cli, Commands, KeyCommands, ServeArgs};
use collector::config::CollectorConfig;
use collector::error::CollectorError;
use collector::rate_limit::RateLimiter;
use collector::sink::InMemorySink;
use collector::validation::Validator;

#[tokio::main]
async fn main() {
    collector::logging::init();

    if let Err(error) = run().await {
        tracing::error!(error = %error, "collector stopped");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CollectorError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve(args) => serve(args).await,
        Commands::Key {
            command: KeyCommands::Generate(args),
        } => generate_key(args).await,
    }
}

async fn generate_key(args: collector::cli::KeyGenerateArgs) -> Result<(), CollectorError> {
    let key = collector::key::generate()?;

    info!(
        site_id = %args.site,
        environment = %args.environment,
        "generated ingest key"
    );
    println!("{key}");
    Ok(())
}

async fn serve(args: ServeArgs) -> Result<(), CollectorError> {
    let config = CollectorConfig::load_from_path(&args.config)?;
    let registry = config.registry()?;
    let host: IpAddr = args.host.parse()?;
    let address = SocketAddr::from((host, args.port));
    let validator = Validator::new().map_err(CollectorError::ValidationSetup)?;
    let sink = InMemorySink::new();
    let policy = collector::security::KeyPolicy::new(registry);
    let app = collector::http::router(validator, sink, policy, RateLimiter::new());

    let listener = tokio::net::TcpListener::bind(address).await?;

    info!(
        service = "collector",
        version = env!("CARGO_PKG_VERSION"),
        host = %args.host,
        port = args.port,
        config_path = %args.config.display(),
        site_count = config.sites.len(),
        "collector started"
    );

    axum::serve(listener, app)
        .await
        .map_err(CollectorError::Serve)
}
