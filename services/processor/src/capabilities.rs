use jsonschema::{Draft, JSONSchema};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityStatus {
    Implemented,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityId {
    PageViews,
    BrowserContext,
    AnonymousVisitors,
    Sessions,
    Dimensions,
    CustomEvents,
    WebVitals,
    Conversions,
    Funnels,
    Geo,
}

impl CapabilityId {
    pub const ALL: [Self; 10] = [
        Self::PageViews,
        Self::BrowserContext,
        Self::AnonymousVisitors,
        Self::Sessions,
        Self::Dimensions,
        Self::CustomEvents,
        Self::WebVitals,
        Self::Conversions,
        Self::Funnels,
        Self::Geo,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PageViews => "page_views",
            Self::BrowserContext => "browser_context",
            Self::AnonymousVisitors => "anonymous_visitors",
            Self::Sessions => "sessions",
            Self::Dimensions => "dimensions",
            Self::CustomEvents => "custom_events",
            Self::WebVitals => "web_vitals",
            Self::Conversions => "conversions",
            Self::Funnels => "funnels",
            Self::Geo => "geo",
        }
    }
}

impl FromStr for CapabilityId {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|capability| capability.as_str() == value)
            .ok_or(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityContract {
    pub id: CapabilityId,
    pub status: CapabilityStatus,
    pub depends_on: Vec<CapabilityId>,
    pub input: serde_json::Value,
    pub raw_event: serde_json::Value,
    pub processor: serde_json::Value,
    pub api: serde_json::Value,
    pub dashboard: serde_json::Value,
    pub security: serde_json::Value,
    pub history: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
struct CapabilityManifest {
    capability_schema_version: u8,
    capabilities: Vec<CapabilityContract>,
}

#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    capabilities: Vec<CapabilityContract>,
}

#[derive(Debug, Error)]
pub enum CapabilityRegistryError {
    #[error("invalid capability manifest JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("capability manifest does not match its JSON Schema: {0}")]
    Schema(String),
    #[error("unsupported capability schema version: {0}")]
    UnsupportedSchemaVersion(u8),
    #[error("capability manifest must contain exactly 10 capabilities")]
    InvalidCount,
    #[error("duplicate capability ID: {0}")]
    DuplicateId(String),
    #[error("capability {0} depends on unknown capability {1}")]
    UnknownDependency(String, String),
    #[error("planned capability {0} exposes a runtime surface")]
    PlannedRuntimeSurface(String),
    #[error("capability dependency graph contains a cycle")]
    DependencyCycle,
}

impl CapabilityRegistry {
    pub fn canonical() -> Result<Self, CapabilityRegistryError> {
        Self::from_json(include_str!(
            "../../../protocol/capabilities/v1/capabilities.json"
        ))
    }

    pub fn from_json(json: &str) -> Result<Self, CapabilityRegistryError> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../protocol/capabilities/v1/capability-contract.schema.json"
        ))?;
        let validator = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .compile(&schema)
            .map_err(|error| CapabilityRegistryError::Schema(error.to_string()))?;
        if let Err(errors) = validator.validate(&value) {
            let message = errors
                .map(|error| error.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(CapabilityRegistryError::Schema(message));
        }
        let manifest: CapabilityManifest = serde_json::from_value(value)?;
        if manifest.capability_schema_version != 1 {
            return Err(CapabilityRegistryError::UnsupportedSchemaVersion(
                manifest.capability_schema_version,
            ));
        }
        if manifest.capabilities.len() != CapabilityId::ALL.len() {
            return Err(CapabilityRegistryError::InvalidCount);
        }
        let ids: HashSet<_> = manifest
            .capabilities
            .iter()
            .map(|capability| capability.id)
            .collect();
        if ids.len() != manifest.capabilities.len() {
            let duplicate = manifest
                .capabilities
                .iter()
                .find(|capability| {
                    manifest
                        .capabilities
                        .iter()
                        .filter(|other| other.id == capability.id)
                        .count()
                        > 1
                })
                .map(|capability| capability.id.as_str())
                .unwrap_or("unknown");
            return Err(CapabilityRegistryError::DuplicateId(duplicate.to_owned()));
        }
        for capability in &manifest.capabilities {
            if capability.status == CapabilityStatus::Planned
                && (!has_string_field(&capability.api, "query_surface", "future_report")
                    || !has_empty_array_field(&capability.api, "routes")
                    || !has_string_field(&capability.dashboard, "surface", "future_report"))
            {
                return Err(CapabilityRegistryError::PlannedRuntimeSurface(
                    capability.id.as_str().to_owned(),
                ));
            }
            for dependency in &capability.depends_on {
                if !ids.contains(dependency) {
                    return Err(CapabilityRegistryError::UnknownDependency(
                        capability.id.as_str().to_owned(),
                        dependency.as_str().to_owned(),
                    ));
                }
            }
        }
        let dependencies: HashMap<_, _> = manifest
            .capabilities
            .iter()
            .map(|capability| (capability.id, capability.depends_on.as_slice()))
            .collect();
        if has_dependency_cycle(&dependencies) {
            return Err(CapabilityRegistryError::DependencyCycle);
        }
        Ok(Self {
            capabilities: manifest.capabilities,
        })
    }

    pub fn get(&self, id: CapabilityId) -> Option<&CapabilityContract> {
        self.capabilities
            .iter()
            .find(|capability| capability.id == id)
    }

    pub fn dependencies(&self, id: CapabilityId) -> &[CapabilityId] {
        self.get(id)
            .map_or(&[], |capability| &capability.depends_on)
    }

    pub fn is_implemented(&self, id: CapabilityId) -> bool {
        matches!(
            self.get(id).map(|capability| capability.status),
            Some(CapabilityStatus::Implemented)
        )
    }
}

fn has_string_field(value: &serde_json::Value, field: &str, expected: &str) -> bool {
    value.get(field).and_then(serde_json::Value::as_str) == Some(expected)
}

fn has_empty_array_field(value: &serde_json::Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(serde_json::Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn has_dependency_cycle(dependencies: &HashMap<CapabilityId, &[CapabilityId]>) -> bool {
    fn visit(
        id: CapabilityId,
        dependencies: &HashMap<CapabilityId, &[CapabilityId]>,
        visiting: &mut HashSet<CapabilityId>,
        visited: &mut HashSet<CapabilityId>,
    ) -> bool {
        if visiting.contains(&id) {
            return true;
        }
        if visited.contains(&id) {
            return false;
        }
        visiting.insert(id);
        if let Some(dependencies_for_id) = dependencies.get(&id) {
            for dependency in *dependencies_for_id {
                if visit(*dependency, dependencies, visiting, visited) {
                    return true;
                }
            }
        }
        visiting.remove(&id);
        visited.insert(id);
        false
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    dependencies
        .keys()
        .copied()
        .any(|id| visit(id, dependencies, &mut visiting, &mut visited))
}

#[cfg(test)]
mod tests {
    use super::{CapabilityId, CapabilityRegistry, CapabilityStatus};

    #[test]
    fn exposes_the_canonical_capability_contract() {
        let registry =
            CapabilityRegistry::canonical().expect("capability manifest should be valid");
        let ids: Vec<_> = CapabilityId::ALL.iter().map(|id| id.as_str()).collect();
        let manifest_ids: Vec<_> = registry
            .capabilities
            .iter()
            .map(|capability| capability.id.as_str())
            .collect();

        assert_eq!(ids.len(), 10);
        assert_eq!(ids, manifest_ids);
        assert_eq!(
            registry.get(CapabilityId::Geo).unwrap().id,
            CapabilityId::Geo
        );
        assert_eq!(
            registry.dependencies(CapabilityId::Sessions),
            &[CapabilityId::AnonymousVisitors]
        );
        assert!(registry.is_implemented(CapabilityId::PageViews));
        assert!(!registry.is_implemented(CapabilityId::CustomEvents));
        assert_eq!(
            registry.get(CapabilityId::CustomEvents).unwrap().status,
            CapabilityStatus::Planned
        );
    }

    #[test]
    fn exposes_unknown_capability_errors() {
        assert!("unknown".parse::<CapabilityId>().is_err());
    }

    #[test]
    fn rejects_invalid_manifest_metadata() {
        let json = include_str!("../../../protocol/capabilities/v1/capabilities.json").replace(
            "\"capability_schema_version\": 1",
            "\"capability_schema_version\": 2",
        );
        assert!(CapabilityRegistry::from_json(&json).is_err());
    }

    #[test]
    fn rejects_unknown_manifest_fields() {
        let mut manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../protocol/capabilities/v1/capabilities.json"
        ))
        .expect("canonical manifest should be valid JSON");
        manifest["unexpected"] = serde_json::Value::Bool(true);
        assert!(CapabilityRegistry::from_json(&manifest.to_string()).is_err());
    }

    #[test]
    fn rejects_semantically_invalid_capability_boundaries() {
        let mut manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../protocol/capabilities/v1/capabilities.json"
        ))
        .expect("canonical manifest should be valid JSON");
        manifest["capabilities"][5]["api"]["routes"] = serde_json::json!(["/runtime"]);
        assert!(matches!(
            CapabilityRegistry::from_json(&manifest.to_string()),
            Err(super::CapabilityRegistryError::PlannedRuntimeSurface(_))
        ));
    }
}
