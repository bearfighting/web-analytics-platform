use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub(crate) struct RawEvent {
    pub(crate) id: i64,
    pub(crate) site_id: String,
    pub(crate) occurred_at: DateTime<Utc>,
    pub(crate) path: String,
}
