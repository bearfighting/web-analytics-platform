use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("processor database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("processed raw event {0} was not updated")]
    RawEventNotUpdated(i64),
    #[error("no active generation exists for site {site_id}")]
    NoActiveGeneration { site_id: String },
    #[error("generation {generation_id} does not exist for site {site_id}")]
    GenerationNotFound {
        site_id: String,
        generation_id: String,
    },
    #[error("generation {generation_id} cannot be rollback target because it is {status}")]
    InvalidRollbackTarget {
        generation_id: String,
        status: String,
    },
}
