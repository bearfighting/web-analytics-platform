use thiserror::Error;

use crate::config::ConfigError;
use crate::geo::GeoError;
use crate::key::KeyGenerationError;
use crate::sink::SinkError;
use crate::validation::ValidationError;

#[derive(Debug, Error)]
pub enum CollectorError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("invalid bind address: {0}")]
    Address(#[from] std::net::AddrParseError),
    #[error("failed to bind collector: {0}")]
    Bind(#[from] std::io::Error),
    #[error("collector server failed: {0}")]
    Serve(std::io::Error),
    #[error("failed to initialize event schema validator: {0}")]
    ValidationSetup(ValidationError),
    #[error("DATABASE_URL must be configured")]
    MissingDatabaseUrl,
    #[error("GeoIP configuration error: {0}")]
    GeoConfiguration(String),
    #[error(transparent)]
    Geo(#[from] GeoError),
    #[error(transparent)]
    Storage(#[from] SinkError),
    #[error("failed to generate ingest key: {0}")]
    KeyGeneration(#[from] KeyGenerationError),
}
