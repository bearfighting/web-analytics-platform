use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct CollectorConfig {
    #[serde(default)]
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
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: i64,
    #[serde(skip)]
    pub ingest_key_digests: Vec<[u8; 32]>,
}

pub const DEFAULT_RATE_LIMIT_PER_MINUTE: i64 = 600;

fn default_rate_limit() -> i64 {
    DEFAULT_RATE_LIMIT_PER_MINUTE
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
            rate_limit_per_minute: DEFAULT_RATE_LIMIT_PER_MINUTE,
            ingest_key_digests: Vec::new(),
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
    #[error("site '{site_id}' environment '{environment}' has an invalid rate limit")]
    InvalidRateLimit {
        site_id: String,
        environment: String,
    },
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
    #[error(
        "site '{site_id}' environment '{environment}' has invalid allowed origin '{origin}': {reason}"
    )]
    InvalidOrigin {
        site_id: String,
        environment: String,
        origin: String,
        reason: String,
    },
    #[error("site '{site_id}' origin '{origin}' is assigned to multiple environments")]
    DuplicateSiteOrigin { site_id: String, origin: String },
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
        let sites = sites
            .into_iter()
            .map(|mut site| {
                if site.ingest_keys.is_empty()
                    || site.ingest_keys.iter().any(|key| key.trim().is_empty())
                {
                    let index = site
                        .ingest_keys
                        .iter()
                        .position(|key| key.trim().is_empty())
                        .unwrap_or_default();
                    return Err(ConfigError::EmptyIngestKey {
                        site_id: site.site_id,
                        environment: site.environment,
                        index,
                    });
                }
                site.ingest_key_digests = site
                    .ingest_keys
                    .iter()
                    .map(|key| Sha256::digest(key.as_bytes()).into())
                    .collect();
                site.ingest_keys.clear();
                Ok(site)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_runtime_sites(sites)
    }

    pub fn from_runtime_sites(sites: Vec<SiteConfig>) -> Result<Self, ConfigError> {
        let mut sites_by_id: HashMap<String, Vec<SiteConfig>> = HashMap::new();
        let mut identities = HashMap::new();
        let mut keys = HashMap::new();
        let mut origins = HashMap::new();

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

            for raw_origin in &site.allowed_origins {
                let origin =
                    normalize_origin(raw_origin).map_err(|reason| ConfigError::InvalidOrigin {
                        site_id: site.site_id.clone(),
                        environment: site.environment.clone(),
                        origin: raw_origin.clone(),
                        reason,
                    })?;
                if let Some(previous_environment) = origins.insert(
                    (site.site_id.clone(), origin.clone()),
                    site.environment.clone(),
                ) && previous_environment != site.environment
                {
                    return Err(ConfigError::DuplicateSiteOrigin {
                        site_id: site.site_id.clone(),
                        origin,
                    });
                }
            }

            if site.rate_limit_per_minute < 1 {
                return Err(ConfigError::InvalidRateLimit {
                    site_id: site.site_id,
                    environment: site.environment,
                });
            }
            for digest in &site.ingest_key_digests {
                if let Some((first_site_id, first_environment)) =
                    keys.insert(*digest, (site.site_id.clone(), site.environment.clone()))
                {
                    return Err(ConfigError::DuplicateIngestKey {
                        first_site_id,
                        first_environment,
                        second_site_id: site.site_id.clone(),
                        second_environment: site.environment.clone(),
                    });
                }
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

    pub fn all_sites(&self) -> Vec<SiteConfig> {
        self.sites_by_id.values().flatten().cloned().collect()
    }

    pub fn site_allows_origin(&self, site: &SiteConfig, origin: &str) -> bool {
        site.allowed_origins
            .iter()
            .filter_map(|configured| normalize_origin(configured).ok())
            .any(|configured| configured == origin)
    }

    pub fn origin_allowed_anywhere(&self, origin: &str) -> bool {
        self.sites_by_id
            .values()
            .flatten()
            .any(|site| site.enabled && self.site_allows_origin(site, origin))
    }
}

pub fn normalize_origin(raw: &str) -> Result<String, String> {
    if raw == "*" {
        return Err("wildcard origins are not allowed".to_owned());
    }

    let url = Url::parse(raw).map_err(|error| error.to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("scheme must be http or https".to_owned());
    }
    if url.host_str().is_none() {
        return Err("origin must include a host".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("user information is not allowed".to_owned());
    }
    if url.path() != "" && url.path() != "/" {
        return Err("path is not allowed".to_owned());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("query and fragment are not allowed".to_owned());
    }

    Ok(url.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{CollectorConfig, ConfigError, SiteRegistry, normalize_origin};

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
            rate_limit_per_minute: super::DEFAULT_RATE_LIMIT_PER_MINUTE,
            ingest_key_digests: Vec::new(),
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

    #[test]
    fn normalizes_and_validates_origins() {
        assert_eq!(
            normalize_origin("https://EXAMPLE.com/").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            normalize_origin("http://localhost:3000").unwrap(),
            "http://localhost:3000"
        );
        assert!(normalize_origin("*").is_err());
        assert!(normalize_origin("https://example.com/path").is_err());
        assert!(normalize_origin("ftp://example.com").is_err());
    }

    #[test]
    fn rejects_duplicate_site_origin_across_environments() {
        let mut first = super::SiteConfig::new("site_example", "development", true, "key-one");
        first.allowed_origins = vec!["https://example.com".into()];
        let mut second = super::SiteConfig::new("site_example", "production", true, "key-two");
        second.allowed_origins = vec!["https://example.com/".into()];

        assert!(matches!(
            SiteRegistry::from_sites(vec![first, second]),
            Err(ConfigError::DuplicateSiteOrigin { .. })
        ));
    }
}
