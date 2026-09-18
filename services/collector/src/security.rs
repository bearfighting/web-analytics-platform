use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;

use crate::config::{SiteConfig, SiteRegistry};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AccessError {
    #[error("site is not allowed")]
    SiteNotAllowed,
    #[error("ingest key is invalid")]
    InvalidIngestKey,
}

#[derive(Clone)]
pub struct KeyPolicy {
    registry: SiteRegistry,
}

impl KeyPolicy {
    pub fn new(registry: SiteRegistry) -> Self {
        Self { registry }
    }

    pub fn authorize(
        &self,
        site_id: &str,
        ingest_key: Option<&str>,
    ) -> Result<&SiteConfig, AccessError> {
        let sites = self
            .registry
            .sites(site_id)
            .ok_or(AccessError::SiteNotAllowed)?;

        if !sites.iter().any(|site| site.enabled) {
            log_rejected(site_id, ingest_key, "disabled site");
            return Err(AccessError::SiteNotAllowed);
        }

        let Some(ingest_key) = ingest_key else {
            log_rejected(site_id, None, "missing ingest key");
            return Err(AccessError::InvalidIngestKey);
        };

        let matched = sites.iter().find(|site| {
            site.enabled
                && site
                    .ingest_keys
                    .iter()
                    .any(|configured| configured.as_bytes().ct_eq(ingest_key.as_bytes()).into())
        });

        match matched {
            Some(site) => {
                tracing::info!(
                    site_id = %site.site_id,
                    environment = %site.environment,
                    key_sha256 = %key_fingerprint(ingest_key),
                    "ingest key accepted"
                );
                Ok(site)
            }
            None => {
                log_rejected(site_id, Some(ingest_key), "invalid ingest key");
                Err(AccessError::InvalidIngestKey)
            }
        }
    }
}

pub fn key_fingerprint(key: &str) -> String {
    let digest = Sha256::digest(key.as_bytes());
    digest[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn log_rejected(site_id: &str, key: Option<&str>, reason: &str) {
    match key {
        Some(key) => tracing::warn!(
            site_id,
            key_sha256 = %key_fingerprint(key),
            reason,
            "ingest key rejected"
        ),
        None => tracing::warn!(site_id, reason, "ingest key rejected"),
    }
}

#[cfg(test)]
mod tests {
    use super::{AccessError, KeyPolicy, key_fingerprint};
    use crate::config::{SiteConfig, SiteRegistry};

    fn policy() -> KeyPolicy {
        KeyPolicy::new(
            SiteRegistry::from_sites(vec![
                SiteConfig::new("site_example", "development", true, "development-key"),
                SiteConfig::new("site_example", "production", true, "production-key"),
                SiteConfig::new("site_disabled", "production", false, "disabled-key"),
            ])
            .expect("test registry should be valid"),
        )
    }

    #[test]
    fn selects_environment_by_key() {
        let policy = policy();
        let site = policy
            .authorize("site_example", Some("production-key"))
            .expect("key should be accepted");
        assert_eq!(site.environment, "production");
    }

    #[test]
    fn rejects_missing_or_invalid_key() {
        assert!(matches!(
            policy().authorize("site_example", None),
            Err(AccessError::InvalidIngestKey)
        ));
        assert!(matches!(
            policy().authorize("site_example", Some("wrong-key")),
            Err(AccessError::InvalidIngestKey)
        ));
    }

    #[test]
    fn rejects_unknown_and_disabled_sites() {
        assert!(matches!(
            policy().authorize("site_unknown", Some("production-key")),
            Err(AccessError::SiteNotAllowed)
        ));
        assert!(matches!(
            policy().authorize("site_disabled", Some("disabled-key")),
            Err(AccessError::SiteNotAllowed)
        ));
    }

    #[test]
    fn fingerprint_is_short_and_does_not_contain_key() {
        let fingerprint = key_fingerprint("production-key");
        assert_eq!(fingerprint.len(), 12);
        assert!(!fingerprint.contains("production-key"));
    }
}
