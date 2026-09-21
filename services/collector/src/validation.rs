use std::sync::Arc;

use jsonschema::{Draft, JSONSchema, SchemaResolver, SchemaResolverError};
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::protocol::PageViewEvent;

#[derive(Debug, Clone)]
pub struct ValidatedBatch {
    pub schema_version: u8,
    pub events: Vec<ValidatedEvent>,
}

#[derive(Debug, Clone)]
pub struct ValidatedEvent {
    pub event: PageViewEvent,
    pub payload: Value,
}

const EVENT_BATCH_SCHEMA: &str = include_str!("../../../protocol/schemas/event-batch.schema.json");
const PAGE_VIEW_SCHEMA: &str =
    include_str!("../../../protocol/schemas/page-view-event.schema.json");
const EVENT_BATCH_V2_SCHEMA: &str =
    include_str!("../../../protocol/phase-5/contract/schemas/event-batch-v2.schema.json");
const PAGE_VIEW_V2_SCHEMA: &str =
    include_str!("../../../protocol/phase-5/contract/schemas/page-view-event-v2.schema.json");
const CONTEXT_V2_SCHEMA: &str =
    include_str!("../../../protocol/phase-5/contract/schemas/browser-context-v1.schema.json");

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("failed to parse embedded event batch schema: {0}")]
    BatchSchema(serde_json::Error),
    #[error("failed to parse embedded page view schema: {0}")]
    PageViewSchema(serde_json::Error),
    #[error("failed to compile event schemas: {0}")]
    Compile(String),
}

#[derive(Debug)]
pub struct Validator {
    batch_v1: JSONSchema,
    event_v1: JSONSchema,
    batch_v2: JSONSchema,
    event_v2: JSONSchema,
}

impl Validator {
    pub fn new() -> Result<Self, ValidationError> {
        let batch_schema: Value =
            serde_json::from_str(EVENT_BATCH_SCHEMA).map_err(ValidationError::BatchSchema)?;
        let page_view_schema: Value =
            serde_json::from_str(PAGE_VIEW_SCHEMA).map_err(ValidationError::PageViewSchema)?;
        let batch_v2_schema: Value =
            serde_json::from_str(EVENT_BATCH_V2_SCHEMA).map_err(ValidationError::BatchSchema)?;
        let page_view_v2_schema: Value =
            serde_json::from_str(PAGE_VIEW_V2_SCHEMA).map_err(ValidationError::PageViewSchema)?;
        let context_v2_schema: Value =
            serde_json::from_str(CONTEXT_V2_SCHEMA).map_err(ValidationError::PageViewSchema)?;

        let batch_v1 = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_resolver(EmbeddedResolver {
                page_view_schema: page_view_schema.clone(),
                context_schema: None,
            })
            .compile(&batch_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;
        let event_v1 = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .compile(&page_view_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;
        let batch_v2 = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_resolver(EmbeddedResolver {
                page_view_schema: page_view_v2_schema.clone(),
                context_schema: Some(context_v2_schema.clone()),
            })
            .compile(&batch_v2_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;
        let event_v2 = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_resolver(EmbeddedResolver {
                page_view_schema: page_view_v2_schema,
                context_schema: Some(context_v2_schema),
            })
            .compile(&page_view_v2_schema_for_compile())
            .map_err(|error| ValidationError::Compile(error.to_string()))?;

        Ok(Self {
            batch_v1,
            event_v1,
            batch_v2,
            event_v2,
        })
    }

    pub fn validate(&self, value: &Value) -> Result<ValidatedBatch, Vec<String>> {
        let schema_version = value
            .get("schema_version")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u8;
        let validator = if schema_version == 2 {
            &self.batch_v2
        } else {
            &self.batch_v1
        };
        if let Err(errors) = validator.validate(value) {
            return Err(errors.map(|error| error.to_string()).collect());
        }

        let events = value
            .get("events")
            .and_then(Value::as_array)
            .ok_or_else(|| vec!["events must be an array".to_owned()])?;
        let mut validated_events = Vec::with_capacity(events.len());
        for payload in events {
            let event =
                serde_json::from_value(payload.clone()).map_err(|error| vec![error.to_string()])?;
            validated_events.push(ValidatedEvent {
                event,
                payload: payload.clone(),
            });
        }

        Ok(ValidatedBatch {
            schema_version,
            events: validated_events,
        })
    }

    #[allow(dead_code)]
    pub fn validate_event(&self, value: &Value) -> Result<PageViewEvent, Vec<String>> {
        let schema_version = value
            .get("schema_version")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let validator = if schema_version == 2 {
            &self.event_v2
        } else {
            &self.event_v1
        };
        if let Err(errors) = validator.validate(value) {
            return Err(errors.map(|error| error.to_string()).collect());
        }

        serde_json::from_value(value.clone()).map_err(|error| vec![error.to_string()])
    }
}

struct EmbeddedResolver {
    page_view_schema: Value,
    context_schema: Option<Value>,
}

impl SchemaResolver for EmbeddedResolver {
    fn resolve(
        &self,
        _root_schema: &Value,
        url: &Url,
        _original_reference: &str,
    ) -> Result<Arc<Value>, SchemaResolverError> {
        if url.as_str() == "https://web-analytics-platform.dev/schemas/page-view-event.schema.json"
            || url.as_str()
                == "https://web-analytics-platform.dev/schemas/phase-5/page-view-event-v2.schema.json"
        {
            return Ok(Arc::new(self.page_view_schema.clone()));
        }
        if url.as_str()
            == "https://web-analytics-platform.dev/schemas/phase-5/browser-context-v1.schema.json"
            && let Some(schema) = &self.context_schema
        {
            return Ok(Arc::new(schema.clone()));
        }

        Err(anyhow::anyhow!("embedded schema not found: {url}"))
    }
}

fn page_view_v2_schema_for_compile() -> Value {
    serde_json::from_str(PAGE_VIEW_V2_SCHEMA)
        .expect("embedded V2 page view schema should be valid JSON")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Validator;

    #[test]
    fn canonical_valid_fixtures_are_accepted() {
        let validator = Validator::new().expect("embedded schemas should compile");
        let directories = [
            fixture_directory("valid"),
            phase5_fixture_directory("valid"),
        ];

        for directory in directories {
            for entry in fs::read_dir(directory).expect("valid fixture directory should exist") {
                let path = entry.expect("fixture entry should be readable").path();
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }

                let value: serde_json::Value = serde_json::from_str(
                    &fs::read_to_string(&path).expect("fixture should be readable"),
                )
                .expect("valid fixture should be JSON");
                let result = if value.get("event_id").is_some() {
                    validator.validate_event(&value).map(|_| ())
                } else {
                    validator.validate(&value).map(|_| ())
                };
                assert!(
                    result.is_ok(),
                    "valid fixture should pass: {}",
                    path.display()
                );
            }
        }
    }

    #[test]
    fn canonical_invalid_fixtures_are_rejected() {
        let validator = Validator::new().expect("embedded schemas should compile");
        let directories = [
            fixture_directory("invalid"),
            phase5_fixture_directory("invalid"),
        ];

        for directory in directories {
            for entry in fs::read_dir(directory).expect("invalid fixture directory should exist") {
                let path = entry.expect("fixture entry should be readable").path();
                if path.extension().and_then(|value| value.to_str()) != Some("json") {
                    continue;
                }

                let value: serde_json::Value = serde_json::from_str(
                    &fs::read_to_string(&path).expect("fixture should be readable"),
                )
                .expect("invalid fixture should still be JSON");
                let result = if value.get("event_id").is_some() {
                    validator.validate_event(&value).map(|_| ())
                } else {
                    validator.validate(&value).map(|_| ())
                };
                assert!(
                    result.is_err(),
                    "invalid fixture should fail: {}",
                    path.display()
                );
            }
        }
    }

    fn fixture_directory(kind: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../protocol/fixtures")
            .join(kind)
    }

    fn phase5_fixture_directory(kind: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../protocol/phase-5/fixtures")
            .join(kind)
    }
}
