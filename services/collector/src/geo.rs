use std::{net::IpAddr, path::Path, sync::Arc};

use ipnet::IpNet;
use maxminddb::{Reader, geoip2};
use serde::Serialize;
use thiserror::Error;

pub const GEO_PROVIDER: &str = "maxmind";
pub const GEO_PARSER_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize)]
pub struct GeoEnrichment {
    pub country_code: String,
    pub provider: String,
    pub dataset_version: String,
    pub parser_version: String,
}

#[derive(Debug, Error)]
pub enum GeoError {
    #[error("failed to open GeoIP country database: {0}")]
    Open(#[from] maxminddb::MaxMindDbError),
    #[error("GeoIP database is not a GeoLite2 Country database")]
    UnsupportedDatabase,
}

#[derive(Clone)]
pub struct GeoLookup {
    reader: Arc<Reader<Vec<u8>>>,
    dataset_version: String,
}

impl GeoLookup {
    pub fn open(path: &Path) -> Result<Self, GeoError> {
        let reader = Reader::open_readfile(path)?;
        let (database_type, build_epoch) = {
            let metadata = reader.metadata();
            (metadata.database_type.clone(), metadata.build_epoch)
        };
        if !database_type.starts_with("GeoLite2-Country") {
            return Err(GeoError::UnsupportedDatabase);
        }
        Ok(Self {
            reader: Arc::new(reader),
            dataset_version: format!("{database_type}-{build_epoch}"),
        })
    }

    pub fn lookup(&self, ip: Option<IpAddr>) -> GeoEnrichment {
        let country_code = ip
            .and_then(|ip| self.reader.lookup(ip).ok())
            .and_then(|lookup| lookup.decode::<geoip2::Country>().ok().flatten())
            .and_then(|country| country.country.iso_code.map(str::to_owned))
            .filter(|code| valid_country_code(code))
            .unwrap_or_else(|| "unknown".to_owned());

        GeoEnrichment {
            country_code,
            provider: GEO_PROVIDER.to_owned(),
            dataset_version: self.dataset_version.clone(),
            parser_version: GEO_PARSER_VERSION.to_owned(),
        }
    }
}

pub fn valid_country_code(code: &str) -> bool {
    code.len() == 2 && code.bytes().all(|byte| byte.is_ascii_uppercase())
}

pub fn client_ip(
    peer: Option<IpAddr>,
    forwarded_for: Option<&str>,
    trusted_proxies: &[IpNet],
) -> Option<IpAddr> {
    let peer = peer?;
    if !trusted_proxies
        .iter()
        .any(|network| network.contains(&peer))
    {
        return Some(peer);
    }

    let Some(forwarded_for) = forwarded_for else {
        return None;
    };
    let chain = forwarded_for
        .split(',')
        .map(str::trim)
        .map(str::parse::<IpAddr>)
        .collect::<Result<Vec<_>, _>>();
    let Ok(chain) = chain else {
        return None;
    };

    chain
        .into_iter()
        .rev()
        .find(|ip| !trusted_proxies.iter().any(|network| network.contains(ip)))
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use ipnet::IpNet;

    use super::{client_ip, valid_country_code};

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    fn networks(values: &[&str]) -> Vec<IpNet> {
        values.iter().map(|value| value.parse().unwrap()).collect()
    }

    #[test]
    fn only_trusted_proxy_forwarding_is_used() {
        let trusted = networks(&["10.0.0.0/8"]);
        assert_eq!(
            client_ip(Some(ip("203.0.113.7")), Some("198.51.100.4"), &trusted),
            Some(ip("203.0.113.7"))
        );
        assert_eq!(
            client_ip(
                Some(ip("10.0.0.2")),
                Some("198.51.100.4, 10.0.0.1"),
                &trusted
            ),
            Some(ip("198.51.100.4"))
        );
        assert_eq!(
            client_ip(Some(ip("10.0.0.2")), Some("not-an-ip"), &trusted),
            None
        );
        assert_eq!(client_ip(Some(ip("10.0.0.2")), None, &trusted), None);
        assert_eq!(
            client_ip(Some(ip("10.0.0.2")), Some("10.0.0.1, 10.0.0.3"), &trusted),
            None
        );
    }

    #[test]
    fn supports_ipv6_and_trusted_proxy_chains() {
        let trusted = networks(&["10.0.0.0/8", "2001:db8::/32"]);
        assert_eq!(
            client_ip(
                Some(ip("2001:db8::2")),
                Some("2001:4860::7, 10.0.0.4"),
                &trusted
            ),
            Some(ip("2001:4860::7"))
        );
        assert_eq!(client_ip(None, Some("198.51.100.1"), &trusted), None);
    }

    #[test]
    fn missing_dataset_fails_initialization() {
        let path =
            std::env::temp_dir().join(format!("missing-geo-country-{}.mmdb", std::process::id()));
        assert!(super::GeoLookup::open(&path).is_err());
    }

    #[test]
    fn validates_iso_alpha_two_country_codes() {
        assert!(valid_country_code("CA"));
        assert!(!valid_country_code("USA"));
        assert!(!valid_country_code("ca"));
    }
}
