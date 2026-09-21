use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeatureFlagError {
    #[error("feature flag database query failed: {0}")]
    Database(#[from] sqlx::Error),
}

#[async_trait]
pub trait FeatureFlagStore: Send + Sync {
    async fn protocol_v2_enabled(&self, site_id: &str) -> Result<bool, FeatureFlagError>;
}

#[derive(Clone, Copy, Default)]
pub struct StaticFeatureFlagStore {
    enabled: bool,
}

impl StaticFeatureFlagStore {
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
    pub fn enabled() -> Self {
        Self { enabled: true }
    }
}

#[async_trait]
impl FeatureFlagStore for StaticFeatureFlagStore {
    async fn protocol_v2_enabled(&self, _site_id: &str) -> Result<bool, FeatureFlagError> {
        Ok(self.enabled)
    }
}
