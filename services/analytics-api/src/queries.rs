use sqlx::PgPool;

use sqlx::{Postgres, Transaction};

use crate::models::{
    ActiveGeneration, DateRange, DimensionRow, PageRow, TimelineRow, VisitorSessionRow,
    WatermarkRow,
};

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

pub(crate) async fn analytics_enabled(
    transaction: &mut Transaction<'_, Postgres>,
    site_id: &str,
) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT analytics_enabled
         FROM analytics_feature_flags
         WHERE site_id = $1",
    )
    .bind(site_id)
    .fetch_optional(&mut **transaction)
    .await?
    .unwrap_or(false))
}

pub(crate) async fn active_generation(
    transaction: &mut Transaction<'_, Postgres>,
    site_id: &str,
) -> Result<Option<ActiveGeneration>, sqlx::Error> {
    sqlx::query_as::<_, (String, i32)>(
        "SELECT generation_id::text, aggregation_version
         FROM analytics_generations
         WHERE site_id = $1 AND status = 'active'",
    )
    .bind(site_id)
    .fetch_optional(&mut **transaction)
    .await
    .map(|row| {
        row.map(|(generation_id, aggregation_version)| ActiveGeneration {
            generation_id,
            aggregation_version,
        })
    })
}

pub(crate) async fn phase6_visitor_counts(
    transaction: &mut Transaction<'_, Postgres>,
    site_id: &str,
    generation_id: &str,
    range: DateRange,
) -> Result<(i64, i64, i64), sqlx::Error> {
    let page_views = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(page_views)::bigint
         FROM page_view_daily
         WHERE site_id = $1 AND day BETWEEN $2 AND $3",
    )
    .bind(site_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_one(&mut **transaction)
    .await?
    .unwrap_or(0);
    let unique_visitors = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT visitor_id)::bigint
         FROM visitor_event_facts
         WHERE site_id = $1 AND generation_id = $2::uuid
           AND day BETWEEN $3 AND $4",
    )
    .bind(site_id)
    .bind(generation_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_one(&mut **transaction)
    .await?;
    let sessions = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(DISTINCT session_id)::bigint
         FROM session_events
         WHERE site_id = $1 AND generation_id = $2::uuid
           AND day BETWEEN $3 AND $4",
    )
    .bind(site_id)
    .bind(generation_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_one(&mut **transaction)
    .await?;
    Ok((page_views, unique_visitors, sessions))
}

pub(crate) async fn visitor_session_daily(
    transaction: &mut Transaction<'_, Postgres>,
    site_id: &str,
    generation_id: &str,
    range: DateRange,
) -> Result<Vec<VisitorSessionRow>, sqlx::Error> {
    sqlx::query_as::<_, VisitorSessionRow>(
        "WITH days AS (
             SELECT day FROM page_view_daily
             WHERE site_id = $1 AND day BETWEEN $2 AND $3
             UNION
             SELECT day FROM visitor_daily
             WHERE site_id = $1 AND generation_id = $4::uuid AND day BETWEEN $2 AND $3
             UNION
             SELECT day FROM session_daily
             WHERE site_id = $1 AND generation_id = $4::uuid AND day BETWEEN $2 AND $3
         )
         SELECT days.day,
                COALESCE((SELECT page_views FROM page_view_daily p
                          WHERE p.site_id = $1 AND p.day = days.day), 0)::bigint AS page_views,
                COALESCE((SELECT unique_visitors FROM visitor_daily v
                          WHERE v.site_id = $1 AND v.generation_id = $4::uuid AND v.day = days.day), 0)::bigint AS unique_visitors,
                COALESCE((SELECT sessions FROM session_daily s
                          WHERE s.site_id = $1 AND s.generation_id = $4::uuid AND s.day = days.day), 0)::bigint AS sessions
         FROM days
         ORDER BY days.day ASC",
    )
    .bind(site_id)
    .bind(range.from)
    .bind(range.to)
    .bind(generation_id)
    .fetch_all(&mut **transaction)
    .await
}

pub(crate) async fn dimension_rows(
    transaction: &mut Transaction<'_, Postgres>,
    site_id: &str,
    generation_id: &str,
    dimension: &str,
    range: DateRange,
    limit: i64,
) -> Result<Vec<DimensionRow>, sqlx::Error> {
    sqlx::query_as::<_, DimensionRow>(
        "SELECT value,
                COUNT(*)::bigint AS page_views,
                COUNT(DISTINCT visitor_id)::bigint AS unique_visitors,
                COUNT(DISTINCT session_id)::bigint AS sessions
         FROM dimension_event_facts
         WHERE site_id = $1 AND generation_id = $2::uuid
           AND dimension = $3 AND day BETWEEN $4 AND $5
         GROUP BY value
         ORDER BY page_views DESC, value ASC
         LIMIT $6",
    )
    .bind(site_id)
    .bind(generation_id)
    .bind(dimension)
    .bind(range.from)
    .bind(range.to)
    .bind(limit)
    .fetch_all(&mut **transaction)
    .await
}

pub(crate) async fn watermarks(
    transaction: &mut Transaction<'_, Postgres>,
    site_id: &str,
    generation_id: &str,
    generation_source: &str,
) -> Result<Vec<WatermarkRow>, sqlx::Error> {
    sqlx::query_as::<_, WatermarkRow>(
        "SELECT source_name, processed_received_watermark
         FROM analytics_watermarks
         WHERE site_id = $1
           AND ((generation_id IS NULL AND source_name = 'page_views')
             OR (generation_id = $2::uuid AND source_name = $3))",
    )
    .bind(site_id)
    .bind(generation_id)
    .bind(generation_source)
    .fetch_all(&mut **transaction)
    .await
}
