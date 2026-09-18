mod cli;
mod config;
mod error;
mod http;
mod logging;

use std::net::{IpAddr, SocketAddr};

use clap::Parser;
use tracing::info;

use crate::cli::{Cli, Commands, ServeArgs};
use crate::config::CollectorConfig;
use crate::error::CollectorError;

#[tokio::main]
async fn main() {
    logging::init();

    if let Err(error) = run().await {
        tracing::error!(error = %error, "collector stopped");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), CollectorError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve(args) => serve(args).await,
    }
}

async fn serve(args: ServeArgs) -> Result<(), CollectorError> {
    let config = CollectorConfig::load_from_path(&args.config)?;
    let host: IpAddr = args.host.parse()?;
    let address = SocketAddr::from((host, args.port));
    let app = http::router();

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
