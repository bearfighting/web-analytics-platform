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

    pub async fn rollback_generation(
        &self,
        site_id: &str,
        target_generation_id: &str,
    ) -> Result<(), ProcessorError> {
        let mut transaction = self.pool.begin().await?;

        let target_status = sqlx::query_scalar::<_, String>(
            "SELECT status
             FROM analytics_generations
             WHERE site_id = $1 AND generation_id = $2::uuid
             FOR UPDATE",
        )
        .bind(site_id)
        .bind(target_generation_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| ProcessorError::GenerationNotFound {
            site_id: site_id.to_owned(),
            generation_id: target_generation_id.to_owned(),
        })?;

        if target_status != "retired" {
            return Err(ProcessorError::InvalidRollbackTarget {
                generation_id: target_generation_id.to_owned(),
                status: target_status,
            });
        }

        let active_generation_id = sqlx::query_scalar::<_, String>(
            "SELECT generation_id::text
             FROM analytics_generations
             WHERE site_id = $1 AND status = 'active'
             FOR UPDATE",
        )
        .bind(site_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| ProcessorError::NoActiveGeneration {
            site_id: site_id.to_owned(),
        })?;

        sqlx::query(
            "UPDATE analytics_generations
             SET status = 'retired'
             WHERE site_id = $1 AND generation_id = $2::uuid AND status = 'active'",
        )
        .bind(site_id)
        .bind(&active_generation_id)
        .execute(&mut *transaction)
        .await?;

        sqlx::query(
            "UPDATE analytics_generations
             SET status = 'active', activated_at = NOW(), failure_reason = NULL
             WHERE site_id = $1 AND generation_id = $2::uuid AND status = 'retired'",
        )
        .bind(site_id)
        .bind(target_generation_id)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(())
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
