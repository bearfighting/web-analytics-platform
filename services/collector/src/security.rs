use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;

use crate::config::{SiteConfig, SiteRegistry, normalize_origin};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AccessError {
    #[error("site is not allowed")]
    SiteNotAllowed,
    #[error("origin is not allowed")]
    OriginNotAllowed,
    #[error("ingest key is invalid")]
    InvalidIngestKey { origin: String },
}

#[derive(Clone)]
pub struct SecurityPolicy {
    registry: SiteRegistry,
}

pub struct AuthorizedSite<'a> {
    pub site: &'a SiteConfig,
    pub origin: String,
}

impl SecurityPolicy {
    pub fn new(registry: SiteRegistry) -> Self {
        Self { registry }
    }

    pub fn authorize(
        &self,
        site_id: &str,
        request_origin: Option<&str>,
        ingest_key: Option<&str>,
    ) -> Result<AuthorizedSite<'_>, AccessError> {
        let sites = self
            .registry
            .sites(site_id)
            .ok_or(AccessError::SiteNotAllowed)?;

        if !sites.iter().any(|site| site.enabled) {
            log_rejected(site_id, ingest_key, "disabled site");
            return Err(AccessError::SiteNotAllowed);
        }

        let Some(request_origin) = request_origin else {
            log_rejected(site_id, ingest_key, "missing origin");
            return Err(AccessError::OriginNotAllowed);
        };
        let origin = normalize_origin(request_origin).map_err(|_| {
            log_rejected(site_id, ingest_key, "invalid origin");
            AccessError::OriginNotAllowed
        })?;

        let origin_sites: Vec<&SiteConfig> = sites
            .iter()
            .filter(|site| site.enabled && self.registry.site_allows_origin(site, &origin))
            .collect();

        if origin_sites.is_empty() {
            log_rejected(site_id, ingest_key, "origin is not allowed");
            return Err(AccessError::OriginNotAllowed);
        }

        let Some(ingest_key) = ingest_key else {
            log_rejected(site_id, None, "missing ingest key");
            return Err(AccessError::InvalidIngestKey { origin });
        };

        let matched = origin_sites.into_iter().find(|site| {
            site.ingest_keys
                .iter()
                .any(|configured| configured.as_bytes().ct_eq(ingest_key.as_bytes()).into())
        });

        match matched {
            Some(site) => {
                tracing::info!(
                    site_id = %site.site_id,
                    environment = %site.environment,
                    key_sha256 = %key_fingerprint(ingest_key),
                    origin = %origin,
                    "ingest key and origin accepted"
                );
                Ok(AuthorizedSite { site, origin })
            }
            None => {
                log_rejected(site_id, Some(ingest_key), "invalid ingest key");
                Err(AccessError::InvalidIngestKey { origin })
            }
        }
    }

    pub fn preflight_origin_allowed(&self, request_origin: &str) -> Option<String> {
        let origin = normalize_origin(request_origin).ok()?;
        self.registry
            .origin_allowed_anywhere(&origin)
            .then_some(origin)
    }
}

pub use SecurityPolicy as KeyPolicy;

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
            "security policy rejected request"
        ),
        None => tracing::warn!(site_id, reason, "security policy rejected request"),
    }
}

#[cfg(test)]
mod tests {
    use super::{AccessError, KeyPolicy, key_fingerprint};
    use crate::config::{SiteConfig, SiteRegistry};

    fn policy() -> KeyPolicy {
        let mut development =
            SiteConfig::new("site_example", "development", true, "development-key");
        development.allowed_origins = vec!["http://localhost:3000".into()];
        let mut production = SiteConfig::new("site_example", "production", true, "production-key");
        production.allowed_origins = vec!["https://example.com".into()];

        KeyPolicy::new(
            SiteRegistry::from_sites(vec![
                development,
                production,
                SiteConfig::new("site_disabled", "production", false, "disabled-key"),
            ])
            .expect("test registry should be valid"),
        )
    }

    #[test]
    fn selects_environment_by_matching_origin_and_key() {
        let policy = policy();
        let site = policy
            .authorize(
                "site_example",
                Some("https://example.com/"),
                Some("production-key"),
            )
            .expect("origin and key should be accepted");
        assert_eq!(site.site.environment, "production");
        assert_eq!(site.origin, "https://example.com");
    }

    #[test]
    fn rejects_missing_or_invalid_key_after_origin_match() {
        assert!(matches!(
            policy().authorize("site_example", Some("https://example.com"), None),
            Err(AccessError::InvalidIngestKey { .. })
        ));
        assert!(matches!(
            policy().authorize(
                "site_example",
                Some("https://example.com"),
                Some("wrong-key")
            ),
            Err(AccessError::InvalidIngestKey { .. })
        ));
    }

    #[test]
    fn rejects_unknown_disabled_and_disallowed_origins() {
        assert!(matches!(
            policy().authorize(
                "site_unknown",
                Some("https://example.com"),
                Some("production-key")
            ),
            Err(AccessError::SiteNotAllowed)
        ));
        assert!(matches!(
            policy().authorize(
                "site_disabled",
                Some("https://example.com"),
                Some("disabled-key")
            ),
            Err(AccessError::SiteNotAllowed)
        ));
        assert!(matches!(
            policy().authorize(
                "site_example",
                Some("https://evil.example"),
                Some("production-key")
            ),
            Err(AccessError::OriginNotAllowed)
        ));
        assert!(matches!(
            policy().authorize("site_example", None, Some("production-key")),
            Err(AccessError::OriginNotAllowed)
        ));
    }

    #[test]
    fn key_cannot_cross_environment_origin() {
        assert!(matches!(
            policy().authorize(
                "site_example",
                Some("https://example.com"),
                Some("development-key")
            ),
            Err(AccessError::InvalidIngestKey { .. })
        ));
    }

    #[test]
    fn preflight_accepts_any_enabled_allowlisted_origin() {
        assert_eq!(
            policy().preflight_origin_allowed("https://EXAMPLE.com/"),
            Some("https://example.com".into())
        );
        assert_eq!(
            policy().preflight_origin_allowed("https://evil.example"),
            None
        );
    }

    #[test]
    fn fingerprint_is_short_and_does_not_contain_key() {
        let fingerprint = key_fingerprint("production-key");
        assert_eq!(fingerprint.len(), 12);
        assert!(!fingerprint.contains("production-key"));
    }
}
