use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgConnection, Postgres, Transaction};

use crate::models::RawEvent;

pub(crate) async fn claim_next_event(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Option<RawEvent>, sqlx::Error> {
    sqlx::query_as::<_, (i64, String, DateTime<Utc>, String)>(
        "SELECT id, site_id, occurred_at, path
         FROM raw_events
         WHERE processed_at IS NULL
         ORDER BY id
         LIMIT 1
         FOR UPDATE SKIP LOCKED",
    )
    .fetch_optional(&mut **transaction)
    .await
    .map(|event| {
        event.map(|(id, site_id, occurred_at, path)| RawEvent {
            id,
            site_id,
            occurred_at,
            path,
        })
    })
}

pub(crate) async fn upsert_daily(
    connection: &mut PgConnection,
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
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub(crate) async fn upsert_route(
    connection: &mut PgConnection,
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
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub(crate) async fn upsert_total(
    connection: &mut PgConnection,
    site_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO page_view_totals (site_id, page_views)
         VALUES ($1, 1)
         ON CONFLICT (site_id)
         DO UPDATE SET page_views = page_view_totals.page_views + 1",
    )
    .bind(site_id)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub(crate) async fn mark_processed(
    connection: &mut PgConnection,
    event_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE raw_events
         SET processed_at = NOW()
         WHERE id = $1 AND processed_at IS NULL",
    )
    .bind(event_id)
    .execute(&mut *connection)
    .await?;
    Ok(result.rows_affected() == 1)
}
