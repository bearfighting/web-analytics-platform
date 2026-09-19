use sqlx::PgPool;

use crate::models::{DateRange, PageRow, TimelineRow};

pub(crate) async fn health(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}

pub(crate) async fn overview(pool: &PgPool, site_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE((SELECT page_views FROM page_view_totals WHERE site_id = $1), 0)",
    )
    .bind(site_id)
    .fetch_one(pool)
    .await
}

pub(crate) async fn range_overview(
    pool: &PgPool,
    site_id: &str,
    range: DateRange,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(page_views)::bigint FROM page_view_daily WHERE site_id = $1 AND day BETWEEN $2 AND $3",
    )
    .bind(site_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_one(pool)
    .await
    .map(|page_views| page_views.unwrap_or(0))
}

pub(crate) async fn timeline(
    pool: &PgPool,
    site_id: &str,
    range: DateRange,
) -> Result<Vec<TimelineRow>, sqlx::Error> {
    sqlx::query_as::<_, TimelineRow>(
        "SELECT day, page_views FROM page_view_daily
         WHERE site_id = $1 AND day BETWEEN $2 AND $3 ORDER BY day ASC",
    )
    .bind(site_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_all(pool)
    .await
}

pub(crate) async fn pages(
    pool: &PgPool,
    site_id: &str,
    range: DateRange,
    limit: i64,
) -> Result<Vec<PageRow>, sqlx::Error> {
    sqlx::query_as::<_, PageRow>(
        "SELECT path, SUM(page_views)::bigint AS page_views
         FROM page_view_routes
         WHERE site_id = $1 AND day BETWEEN $2 AND $3
         GROUP BY path ORDER BY page_views DESC, path ASC LIMIT $4",
    )
    .bind(site_id)
    .bind(range.from)
    .bind(range.to)
    .bind(limit)
    .fetch_all(pool)
    .await
}
