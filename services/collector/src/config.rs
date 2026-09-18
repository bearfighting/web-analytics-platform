use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct CollectorConfig {
    pub sites: Vec<SiteConfig>,
}

// These fields are parsed in the foundation PR and consumed by later security
// and ingestion layers.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    pub site_id: String,
    pub environment: String,
    pub enabled: bool,
    pub allowed_origins: Vec<String>,
    pub ingest_keys: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config '{path}': {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config '{path}': {source}")]
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("config must contain at least one site")]
    Empty,
    #[error("site_id and environment must not be empty")]
    EmptyIdentity,
}

impl CollectorConfig {
    pub fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let config = toml::from_str::<Self>(&contents).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

        config.validate()
    }

    fn validate(self) -> Result<Self, ConfigError> {
        if self.sites.is_empty() {
            return Err(ConfigError::Empty);
        }

        if self
            .sites
            .iter()
            .any(|site| site.site_id.trim().is_empty() || site.environment.trim().is_empty())
        {
            return Err(ConfigError::EmptyIdentity);
        }

        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{CollectorConfig, ConfigError};

    static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

    fn write_config(contents: &str) -> PathBuf {
        let file_number = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "web-analytics-collector-config-{}-{file_number}.toml",
            std::process::id()
        ));
        fs::write(&path, contents).expect("test config should be writable");
        path
    }

    #[test]
    fn loads_multiple_sites_and_environments() {
        let path = write_config(
            r#"
                [[sites]]
                site_id = "site_example"
                environment = "development"
                enabled = true
                allowed_origins = ["http://localhost:3000"]
                ingest_keys = ["development-key"]

                [[sites]]
                site_id = "site_example"
                environment = "production"
                enabled = true
                allowed_origins = ["https://www.example.com"]
                ingest_keys = ["production-key"]
            "#,
        );

        let config = CollectorConfig::load_from_path(&path).expect("config should load");
        assert_eq!(config.sites.len(), 2);
        assert_eq!(config.sites[1].environment, "production");

        fs::remove_file(path).expect("test config should be removed");
    }

    #[test]
    fn rejects_missing_file() {
        let error = CollectorConfig::load_from_path(PathBuf::from("missing.toml").as_path())
            .expect_err("missing config should fail");

        assert!(matches!(error, ConfigError::Read { .. }));
    }

    #[test]
    fn rejects_invalid_toml() {
        let path = write_config("[[sites]\n");

        let error = CollectorConfig::load_from_path(&path).expect_err("invalid TOML should fail");
        assert!(matches!(error, ConfigError::Parse { .. }));

        fs::remove_file(path).expect("test config should be removed");
    }

    #[test]
    fn rejects_missing_required_field() {
        let path = write_config(
            r#"
                [[sites]]
                site_id = "site_example"
                environment = "development"
                enabled = true
                allowed_origins = []
            "#,
        );

        let error = CollectorConfig::load_from_path(&path).expect_err("missing key should fail");
        assert!(matches!(error, ConfigError::Parse { .. }));

        fs::remove_file(path).expect("test config should be removed");
    }

    #[test]
    fn rejects_wrong_field_type() {
        let path = write_config(
            r#"
                [[sites]]
                site_id = "site_example"
                environment = "development"
                enabled = "yes"
                allowed_origins = []
                ingest_keys = []
            "#,
        );

        let error = CollectorConfig::load_from_path(&path).expect_err("wrong type should fail");
        assert!(matches!(error, ConfigError::Parse { .. }));

        fs::remove_file(path).expect("test config should be removed");
    }
}
