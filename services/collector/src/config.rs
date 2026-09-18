use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

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

impl SiteConfig {
    #[cfg(test)]
    pub fn new(site_id: &str, environment: &str, enabled: bool, ingest_key: &str) -> Self {
        Self {
            site_id: site_id.to_owned(),
            environment: environment.to_owned(),
            enabled,
            allowed_origins: Vec::new(),
            ingest_keys: vec![ingest_key.to_owned()],
        }
    }
}

#[derive(Clone, Debug)]
pub struct SiteRegistry {
    sites_by_id: HashMap<String, Vec<SiteConfig>>,
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
    #[error("site '{site_id}' environment '{environment}' is duplicated")]
    DuplicateSiteEnvironment {
        site_id: String,
        environment: String,
    },
    #[error(
        "site '{site_id}' environment '{environment}' has an empty ingest key at index {index}"
    )]
    EmptyIngestKey {
        site_id: String,
        environment: String,
        index: usize,
    },
    #[error(
        "an ingest key is bound to multiple site environments: '{first_site_id}/{first_environment}' and '{second_site_id}/{second_environment}'"
    )]
    DuplicateIngestKey {
        first_site_id: String,
        first_environment: String,
        second_site_id: String,
        second_environment: String,
    },
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

        config.validate()?;
        config.registry()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
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

        Ok(())
    }

    pub fn registry(&self) -> Result<SiteRegistry, ConfigError> {
        self.validate()?;
        SiteRegistry::from_sites(self.sites.clone())
    }
}

impl SiteRegistry {
    pub fn from_sites(sites: Vec<SiteConfig>) -> Result<Self, ConfigError> {
        let mut sites_by_id: HashMap<String, Vec<SiteConfig>> = HashMap::new();
        let mut identities = HashMap::new();
        let mut keys = HashMap::new();

        for site in sites {
            if site.site_id.trim().is_empty() || site.environment.trim().is_empty() {
                return Err(ConfigError::EmptyIdentity);
            }

            let identity = (site.site_id.clone(), site.environment.clone());
            if identities.insert(identity.clone(), ()).is_some() {
                return Err(ConfigError::DuplicateSiteEnvironment {
                    site_id: site.site_id,
                    environment: site.environment,
                });
            }

            for (index, key) in site.ingest_keys.iter().enumerate() {
                if key.trim().is_empty() {
                    return Err(ConfigError::EmptyIngestKey {
                        site_id: site.site_id.clone(),
                        environment: site.environment.clone(),
                        index,
                    });
                }
                if let Some((first_site_id, first_environment)) = keys.insert(
                    key.clone(),
                    (site.site_id.clone(), site.environment.clone()),
                ) {
                    return Err(ConfigError::DuplicateIngestKey {
                        first_site_id,
                        first_environment,
                        second_site_id: site.site_id.clone(),
                        second_environment: site.environment.clone(),
                    });
                }
            }

            if site.ingest_keys.is_empty() {
                return Err(ConfigError::EmptyIngestKey {
                    site_id: site.site_id.clone(),
                    environment: site.environment.clone(),
                    index: 0,
                });
            }

            sites_by_id
                .entry(site.site_id.clone())
                .or_default()
                .push(site);
        }

        Ok(Self { sites_by_id })
    }

    pub fn sites(&self, site_id: &str) -> Option<&[SiteConfig]> {
        self.sites_by_id.get(site_id).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{CollectorConfig, ConfigError, SiteRegistry};

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

    #[test]
    fn rejects_duplicate_site_environment() {
        let result = SiteRegistry::from_sites(vec![
            super::SiteConfig::new("site_example", "production", true, "key-one"),
            super::SiteConfig::new("site_example", "production", true, "key-two"),
        ]);
        assert!(matches!(
            result,
            Err(ConfigError::DuplicateSiteEnvironment { .. })
        ));
    }

    #[test]
    fn rejects_empty_and_duplicate_ingest_keys() {
        let empty = SiteRegistry::from_sites(vec![super::SiteConfig {
            site_id: "site_example".into(),
            environment: "production".into(),
            enabled: true,
            allowed_origins: vec![],
            ingest_keys: vec![],
        }]);
        assert!(matches!(empty, Err(ConfigError::EmptyIngestKey { .. })));

        let duplicate = SiteRegistry::from_sites(vec![
            super::SiteConfig::new("site_example", "production", true, "same-key"),
            super::SiteConfig::new("site_other", "production", true, "same-key"),
        ]);
        assert!(matches!(
            duplicate,
            Err(ConfigError::DuplicateIngestKey { .. })
        ));
    }

    #[test]
    fn registry_rejects_empty_identity_when_constructed_directly() {
        let result =
            SiteRegistry::from_sites(vec![super::SiteConfig::new(" ", "production", true, "key")]);
        assert!(matches!(result, Err(ConfigError::EmptyIdentity)));
    }
}
