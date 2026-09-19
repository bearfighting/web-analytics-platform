use std::sync::Arc;

use jsonschema::{Draft, JSONSchema, SchemaResolver, SchemaResolverError};
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::protocol::PageViewEvent;

#[derive(Debug, Clone)]
pub struct ValidatedBatch {
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
    batch: JSONSchema,
    #[allow(dead_code)]
    event: JSONSchema,
}

impl Validator {
    pub fn new() -> Result<Self, ValidationError> {
        let batch_schema: Value =
            serde_json::from_str(EVENT_BATCH_SCHEMA).map_err(ValidationError::BatchSchema)?;
        let page_view_schema: Value =
            serde_json::from_str(PAGE_VIEW_SCHEMA).map_err(ValidationError::PageViewSchema)?;

        let batch = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_resolver(EmbeddedResolver {
                page_view_schema: page_view_schema.clone(),
            })
            .compile(&batch_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;
        let event = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .compile(&page_view_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;

        Ok(Self { batch, event })
    }

    pub fn validate(&self, value: &Value) -> Result<ValidatedBatch, Vec<String>> {
        if let Err(errors) = self.batch.validate(value) {
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
            events: validated_events,
        })
    }

    #[allow(dead_code)]
    pub fn validate_event(&self, value: &Value) -> Result<PageViewEvent, Vec<String>> {
        if let Err(errors) = self.event.validate(value) {
            return Err(errors.map(|error| error.to_string()).collect());
        }

        serde_json::from_value(value.clone()).map_err(|error| vec![error.to_string()])
    }
}

struct EmbeddedResolver {
    page_view_schema: Value,
}

impl SchemaResolver for EmbeddedResolver {
    fn resolve(
        &self,
        _root_schema: &Value,
        url: &Url,
        _original_reference: &str,
    ) -> Result<Arc<Value>, SchemaResolverError> {
        if url.as_str() == "https://web-analytics-platform.dev/schemas/page-view-event.schema.json"
        {
            return Ok(Arc::new(self.page_view_schema.clone()));
        }

        Err(anyhow::anyhow!("embedded schema not found: {url}"))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Validator;

    #[test]
    fn canonical_valid_fixtures_are_accepted() {
        let validator = Validator::new().expect("embedded schemas should compile");
        let directory = fixture_directory("valid");

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

    #[test]
    fn canonical_invalid_fixtures_are_rejected() {
        let validator = Validator::new().expect("embedded schemas should compile");
        let directory = fixture_directory("invalid");

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

    fn fixture_directory(kind: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../protocol/fixtures")
            .join(kind)
    }
}
