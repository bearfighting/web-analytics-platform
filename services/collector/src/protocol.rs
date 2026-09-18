use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EventBatch {
    pub schema_version: u8,
    pub events: Vec<PageViewEvent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PageViewEvent {
    pub schema_version: u8,
    pub event_id: String,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub site_id: String,
    pub occurred_at: i64,
    pub path: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub referrer: Option<String>,
    pub context: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    PageView,
}
