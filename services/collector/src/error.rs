use thiserror::Error;

use crate::config::ConfigError;

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
}
