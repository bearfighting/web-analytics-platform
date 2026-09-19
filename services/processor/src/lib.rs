use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error("processor database operation failed: {0}")]
    Database(#[from] sqlx::Error),
    #[error("processed raw event {0} was not updated")]
    RawEventNotUpdated(i64),
}

#[derive(Debug, Clone)]
struct RawEvent {
    id: i64,
    site_id: String,
    occurred_at: DateTime<Utc>,
    path: String,
}

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
        let event = sqlx::query_as::<_, (i64, String, DateTime<Utc>, String)>(
            "SELECT id, site_id, occurred_at, path
             FROM raw_events
             WHERE processed_at IS NULL
             ORDER BY id
             LIMIT 1
             FOR UPDATE SKIP LOCKED",
        )
        .fetch_optional(&mut *transaction)
        .await?
        .map(|(id, site_id, occurred_at, path)| RawEvent {
            id,
            site_id,
            occurred_at,
            path,
        });

        let Some(event) = event else {
            transaction.rollback().await?;
            return Ok(false);
        };

        let day = event.occurred_at.date_naive();
        upsert_daily(&mut transaction, &event.site_id, day).await?;
        upsert_route(&mut transaction, &event.site_id, day, &event.path).await?;
        upsert_total(&mut transaction, &event.site_id).await?;

        let updated = sqlx::query(
            "UPDATE raw_events
             SET processed_at = NOW()
             WHERE id = $1 AND processed_at IS NULL",
        )
        .bind(event.id)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(ProcessorError::RawEventNotUpdated(event.id));
        }

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

async fn upsert_daily(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    site_id: &str,
    day: NaiveDate,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO page_view_daily (site_id, day, page_views)
         VALUES ($1, $2, 1)
         ON CONFLICT (site_id, day)
         DO UPDATE SET page_views = page_view_daily.page_views + 1",
    )
    .bind(site_id)
    .bind(day)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn upsert_route(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    site_id: &str,
    day: NaiveDate,
    path: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO page_view_routes (site_id, day, path, page_views)
         VALUES ($1, $2, $3, 1)
         ON CONFLICT (site_id, day, path)
         DO UPDATE SET page_views = page_view_routes.page_views + 1",
    )
    .bind(site_id)
    .bind(day)
    .bind(path)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn upsert_total(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    site_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO page_view_totals (site_id, page_views)
         VALUES ($1, 1)
         ON CONFLICT (site_id)
         DO UPDATE SET page_views = page_view_totals.page_views + 1",
    )
    .bind(site_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
