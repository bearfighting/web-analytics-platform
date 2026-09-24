use std::{
    net::{IpAddr, SocketAddr},
    path::Path,
};

use clap::Parser;
use tracing::info;

use collector::cli::{Cli, Commands, KeyCommands, ServeArgs};
use collector::config::CollectorConfig;
use collector::error::CollectorError;
use collector::rate_limit::RateLimiter;
use collector::sink::PostgresSink;
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

async fn generate_key(_args: collector::cli::KeyGenerateArgs) -> Result<(), CollectorError> {
    let key = collector::key::generate()?;

    println!("{key}");
    Ok(())
}

async fn serve(args: ServeArgs) -> Result<(), CollectorError> {
    let config = CollectorConfig::load_from_path(&args.config)?;
    let registry = config.registry()?;
    let host: IpAddr = args.host.parse()?;
    let address = SocketAddr::from((host, args.port));
    let validator = Validator::new().map_err(CollectorError::ValidationSetup)?;
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| CollectorError::MissingDatabaseUrl)?;
    let sink = PostgresSink::connect(&database_url).await?;
    let policy = collector::security::KeyPolicy::new(registry);
    let geo_path = std::env::var("GEOIP_DATABASE_PATH").map_err(|_| {
        CollectorError::GeoConfiguration(
            "GEOIP_DATABASE_PATH must point to a supported local GeoLite2 Country or DB-IP City Lite MMDB".to_owned(),
        )
    })?;
    let geo = collector::geo::GeoLookup::open(Path::new(&geo_path))?;
    let trusted_proxies = std::env::var("GEOIP_TRUSTED_PROXIES")
        .unwrap_or_default()
        .split(',')
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().parse::<ipnet::IpNet>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CollectorError::GeoConfiguration(format!("invalid GEOIP_TRUSTED_PROXIES: {error}"))
        })?;
    let app = collector::http::router_with_geo(
        validator,
        sink.clone(),
        policy,
        RateLimiter::new(),
        Some(geo),
        trusted_proxies,
    );

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

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(CollectorError::Serve)
}
