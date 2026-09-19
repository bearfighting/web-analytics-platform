use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("processor database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("processed raw event {0} was not updated")]
    RawEventNotUpdated(i64),
}
