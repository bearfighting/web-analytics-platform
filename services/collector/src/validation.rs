use std::sync::Arc;

use jsonschema::{Draft, JSONSchema, SchemaResolver, SchemaResolverError};
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::protocol::{AnalyticsEvent, CustomEvent, PageViewEvent};

#[derive(Debug, Clone)]
pub struct ValidatedBatch {
    pub schema_version: u8,
    pub events: Vec<ValidatedEvent>,
}

#[derive(Debug, Clone)]
pub struct ValidatedEvent {
    pub event: AnalyticsEvent,
    pub payload: Value,
}

const EVENT_BATCH_SCHEMA: &str =
    include_str!("../../../protocol/events/schemas/event-batch.schema.json");
const PAGE_VIEW_SCHEMA: &str =
    include_str!("../../../protocol/events/schemas/page-view-event.schema.json");
const CUSTOM_EVENT_SCHEMA: &str =
    include_str!("../../../protocol/events/schemas/custom-event.schema.json");
const CONTEXT_SCHEMA: &str = include_str!("../../../protocol/contexts/browser-context.schema.json");

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
    page_view: JSONSchema,
    custom_event: JSONSchema,
}

impl Validator {
    pub fn new() -> Result<Self, ValidationError> {
        let batch_schema: Value =
            serde_json::from_str(EVENT_BATCH_SCHEMA).map_err(ValidationError::BatchSchema)?;
        let page_view_schema: Value =
            serde_json::from_str(PAGE_VIEW_SCHEMA).map_err(ValidationError::PageViewSchema)?;
        let custom_event_schema: Value =
            serde_json::from_str(CUSTOM_EVENT_SCHEMA).map_err(ValidationError::PageViewSchema)?;
        let context_schema: Value =
            serde_json::from_str(CONTEXT_SCHEMA).map_err(ValidationError::PageViewSchema)?;

        let batch = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_resolver(EmbeddedResolver {
                page_view_schema: page_view_schema.clone(),
                custom_event_schema: custom_event_schema.clone(),
                context_schema: context_schema.clone(),
            })
            .compile(&batch_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;
        let page_view = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_resolver(EmbeddedResolver {
                page_view_schema: page_view_schema.clone(),
                custom_event_schema: custom_event_schema.clone(),
                context_schema: context_schema.clone(),
            })
            .compile(&page_view_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;
        let custom_event = JSONSchema::options()
            .with_draft(Draft::Draft202012)
            .should_validate_formats(true)
            .with_resolver(EmbeddedResolver {
                page_view_schema: page_view_schema.clone(),
                custom_event_schema: custom_event_schema.clone(),
                context_schema: context_schema.clone(),
            })
            .compile(&custom_event_schema)
            .map_err(|error| ValidationError::Compile(error.to_string()))?;

        Ok(Self {
            batch,
            page_view,
            custom_event,
        })
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
            let event = self.validate_event(payload)?;
            validated_events.push(ValidatedEvent {
                event,
                payload: payload.clone(),
            });
        }

        Ok(ValidatedBatch {
            schema_version: 1,
            events: validated_events,
        })
    }

    #[allow(dead_code)]
    pub fn validate_event(&self, value: &Value) -> Result<AnalyticsEvent, Vec<String>> {
        let event_type = value.get("type").and_then(Value::as_str).unwrap_or("");
        let schema = if event_type == "page_view" {
            &self.page_view
        } else {
            &self.custom_event
        };
        if let Err(errors) = schema.validate(value) {
            return Err(errors.map(|error| error.to_string()).collect());
        }
        if event_type == "page_view" {
            serde_json::from_value::<PageViewEvent>(value.clone())
                .map(AnalyticsEvent::PageView)
                .map_err(|e| vec![e.to_string()])
        } else {
            let event = serde_json::from_value::<CustomEvent>(value.clone())
                .map_err(|e| vec![e.to_string()])?;
            event.validate_properties().map_err(|e| vec![e])?;
            Ok(AnalyticsEvent::Custom(event))
        }
    }
}

struct EmbeddedResolver {
    page_view_schema: Value,
    custom_event_schema: Value,
    context_schema: Value,
}

impl SchemaResolver for EmbeddedResolver {
    fn resolve(
        &self,
        _root_schema: &Value,
        url: &Url,
        _original_reference: &str,
    ) -> Result<Arc<Value>, SchemaResolverError> {
        if url.as_str()
            == "https://web-analytics-platform.dev/schemas/events/page-view-event.schema.json"
        {
            return Ok(Arc::new(self.page_view_schema.clone()));
        }
        if url.as_str()
            == "https://web-analytics-platform.dev/schemas/events/custom-event.schema.json"
        {
            return Ok(Arc::new(self.custom_event_schema.clone()));
        }
        if url.as_str()
            == "https://web-analytics-platform.dev/schemas/contexts/browser-context.schema.json"
        {
            return Ok(Arc::new(self.context_schema.clone()));
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
        let directories = [fixture_directory("valid")];

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
        let directories = [fixture_directory("invalid")];

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
            .join("../../protocol/events/fixtures")
            .join(kind)
    }
}
