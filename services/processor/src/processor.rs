use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};

use crate::{ProcessorError, models::RawEvent, queries};

#[derive(Clone)]
pub struct Processor {
    pool: PgPool,
}

impl Processor {
    pub async fn connect(database_url: &str) -> Result<Self, ProcessorError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn process_one(&self) -> Result<bool, ProcessorError> {
        let mut transaction = self.pool.begin().await?;
        let Some(event) = queries::claim_next_event(&mut transaction).await? else {
            transaction.rollback().await?;
            return Ok(false);
        };

        process_event(&mut transaction, &event).await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn process_all_once(&self) -> Result<u64, ProcessorError> {
        let mut processed = 0;
        while self.process_one().await? {
            processed += 1;
        }
        Ok(processed)
    }
}

async fn process_event(
    transaction: &mut Transaction<'_, Postgres>,
    event: &RawEvent,
) -> Result<(), ProcessorError> {
    let day = event.occurred_at.with_timezone(&Utc).date_naive();
    queries::upsert_daily(transaction, &event.site_id, day).await?;
    queries::upsert_route(transaction, &event.site_id, day, &event.path).await?;
    queries::upsert_total(transaction, &event.site_id).await?;

    if !queries::mark_processed(transaction, event.id).await? {
        return Err(ProcessorError::RawEventNotUpdated(event.id));
    }
    Ok(())
}
