use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use thiserror::Error;
use tokio::sync::RwLock;

use crate::protocol::{EventType, PageViewEvent};

#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub event: PageViewEvent,
    pub payload: Value,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum SinkError {
    #[error("event timestamp is outside the supported range")]
    InvalidTimestamp,
    #[error("event sink database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("database migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("event sink failed")]
    Failed,
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn accept(&self, events: Vec<StoredEvent>) -> Result<(), SinkError>;
}

#[derive(Clone, Default)]
pub struct InMemorySink {
    events: Arc<RwLock<Vec<PageViewEvent>>>,
}

impl InMemorySink {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub async fn snapshot(&self) -> Vec<PageViewEvent> {
        self.events.read().await.clone()
    }
}

#[async_trait]
impl EventSink for InMemorySink {
    async fn accept(&self, events: Vec<StoredEvent>) -> Result<(), SinkError> {
        self.events
            .write()
            .await
            .extend(events.into_iter().map(|event| event.event));
        Ok(())
    }
}

#[derive(Clone)]
pub struct PostgresSink {
    pool: PgPool,
}

impl PostgresSink {
    pub async fn connect(database_url: &str) -> Result<Self, SinkError> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(database_url: &str) -> Result<(), SinkError> {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        pool.close().await;
        Ok(())
    }
}

#[async_trait]
impl EventSink for PostgresSink {
    async fn accept(&self, events: Vec<StoredEvent>) -> Result<(), SinkError> {
        let mut transaction = self.pool.begin().await?;

        for stored in events {
            let occurred_at = DateTime::<Utc>::from_timestamp_millis(stored.event.occurred_at)
                .ok_or(SinkError::InvalidTimestamp)?;
            let event_type = match stored.event.event_type {
                EventType::PageView => "page_view",
            };

            sqlx::query(
                "INSERT INTO raw_events
                    (site_id, event_id, schema_version, event_type, occurred_at,
                     received_at, path, url, title, referrer, payload)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (site_id, event_id) DO NOTHING",
            )
            .bind(stored.event.site_id)
            .bind(stored.event.event_id)
            .bind(i32::from(stored.event.schema_version))
            .bind(event_type)
            .bind(occurred_at)
            .bind(stored.received_at)
            .bind(stored.event.path)
            .bind(stored.event.url)
            .bind(stored.event.title)
            .bind(stored.event.referrer)
            .bind(stored.payload)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }
}
