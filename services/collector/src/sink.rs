use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::protocol::PageViewEvent;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum SinkError {
    #[error("event sink failed")]
    Failed,
}

#[async_trait]
pub trait EventSink: Send + Sync {
    async fn accept(&self, events: Vec<PageViewEvent>) -> Result<(), SinkError>;
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
    async fn accept(&self, events: Vec<PageViewEvent>) -> Result<(), SinkError> {
        self.events.write().await.extend(events);
        Ok(())
    }
}
