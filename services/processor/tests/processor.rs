use chrono::{DateTime, NaiveDate, Utc};
use processor::Processor;
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point to the integration PostgreSQL database")
}

async fn setup() -> (Processor, PgPool) {
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url())
        .await
        .expect("integration database should be reachable");
    sqlx::query(
        "TRUNCATE analytics_rebuild_queue, dimension_event_facts, dimension_daily,
            normalized_event_context,
            session_events, sessions, visitor_event_facts, session_daily,
            visitor_daily, analytics_watermarks, analytics_generations,
            analytics_feature_flags, raw_events, page_view_daily,
            page_view_routes, page_view_totals RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("aggregate tables should be writable");
    let processor = Processor::connect(&database_url())
        .await
        .expect("processor should connect");
    (processor, pool)
}

async fn insert_raw_event(
    pool: &PgPool,
    id: &str,
    site_id: &str,
    occurred_at: DateTime<Utc>,
    path: &str,
) {
    sqlx::query(
        "INSERT INTO raw_events
            (site_id, event_id, schema_version, event_type, occurred_at,
             received_at, path, payload)
         VALUES ($1, $2, 1, 'page_view', $3, NOW(), $4, $5)",
    )
    .bind(site_id)
    .bind(id)
    .bind(occurred_at)
    .bind(path)
    .bind(json!({
        "schema_version": 1,
        "event_id": id,
        "type": "page_view",
        "site_id": site_id,
        "occurred_at": occurred_at.timestamp_millis(),
        "path": path
    }))
    .execute(pool)
    .await
    .expect("raw event should be insertable");
}

async fn insert_identified_event(
    pool: &PgPool,
    id: &str,
    site_id: &str,
    visitor_id: &str,
    occurred_at: DateTime<Utc>,
    path: &str,
) {
    sqlx::query(
        "INSERT INTO raw_events
            (site_id, event_id, schema_version, event_type, occurred_at,
             received_at, path, payload, visitor_id, context_schema_version)
         VALUES ($1, $2, 1, 'page_view', $3, $3 + INTERVAL '1 minute', $4, $5, $6::uuid, 1)",
    )
    .bind(site_id)
    .bind(id)
    .bind(occurred_at)
    .bind(path)
    .bind(json!({
        "schema_version": 1,
        "event_id": id,
        "type": "page_view",
        "site_id": site_id,
        "visitor_id": visitor_id,
        "occurred_at": occurred_at.timestamp_millis(),
        "path": path,
        "context_schema_version": 1,
        "context": {
            "language": "en-CA",
            "timezone": "America/Toronto",
            "viewport_width": 1280,
            "viewport_height": 720,
            "screen_width": 1920,
            "screen_height": 1080,
            "referrer": "https://example.com/previous",
            "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0"
        }
    }))
    .bind(visitor_id)
    .execute(pool)
    .await
    .expect("identified raw event should be insertable");
}

async fn cleanup_phase6_metadata(pool: &PgPool) {
    for table in [
        "analytics_watermarks",
        "analytics_generations",
        "analytics_feature_flags",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(pool)
            .await
            .expect("phase 6 metadata should be cleanable");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn rollback_generation_is_atomic_and_selects_only_retired_targets() {
    let (processor, pool) = setup().await;
    cleanup_phase6_metadata(&pool).await;

    let active_generation = "00000000-0000-4000-8000-000000000011";
    let retired_generation = "00000000-0000-4000-8000-000000000012";
    sqlx::query(
        "INSERT INTO analytics_generations
            (generation_id, site_id, aggregation_version, parser_version,
             rebuild_reason, status)
         VALUES
            ($1::uuid, 'site_processor', 1, 'woothee-0.13.0', 'initial', 'active'),
            ($2::uuid, 'site_processor', 1, 'woothee-0.13.0', 'backfill', 'retired')",
    )
    .bind(active_generation)
    .bind(retired_generation)
    .execute(&pool)
    .await
    .expect("rollback generations should be insertable");

    processor
        .rollback_generation("site_processor", retired_generation)
        .await
        .expect("retired generation should become active");

    let statuses = sqlx::query(
        "SELECT generation_id::text AS generation_id, status
         FROM analytics_generations
         WHERE site_id = 'site_processor'
         ORDER BY generation_id",
    )
    .fetch_all(&pool)
    .await
    .expect("generation statuses should be queryable");
    assert_eq!(statuses.len(), 2);
    assert_eq!(statuses[0].get::<String, _>("status"), "retired");
    assert_eq!(statuses[1].get::<String, _>("status"), "active");

    let result = processor
        .rollback_generation("site_processor", retired_generation)
        .await;
    assert!(result.is_err());

    let active_count = sqlx::query(
        "SELECT COUNT(*) AS count
         FROM analytics_generations
         WHERE site_id = 'site_processor' AND status = 'active'",
    )
    .fetch_one(&pool)
    .await
    .expect("active generation count should be queryable")
    .get::<i64, _>("count");
    assert_eq!(active_count, 1);

    cleanup_phase6_metadata(&pool).await;
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn rebuild_writes_generation_facts_without_mutating_raw_payload() {
    let (processor, pool) = setup().await;
    let site_id = "site_phase6_processor";
    let visitor_id = "550e8400-e29b-41d4-a716-446655440000";
    let second_visitor_id = "550e8400-e29b-41d4-a716-446655440001";
    sqlx::query(
        "INSERT INTO analytics_feature_flags (site_id, analytics_enabled)
         VALUES ($1, TRUE)",
    )
    .bind(site_id)
    .execute(&pool)
    .await
    .unwrap();

    let first = DateTime::parse_from_rfc3339("2026-09-18T23:30:00Z")
        .unwrap()
        .with_timezone(&Utc);
    insert_identified_event(
        &pool,
        "01J00000000000000000000020",
        site_id,
        visitor_id,
        first,
        "/first",
    )
    .await;
    insert_identified_event(
        &pool,
        "01J00000000000000000000021",
        site_id,
        visitor_id,
        first + chrono::Duration::minutes(10),
        "/same-session",
    )
    .await;
    insert_identified_event(
        &pool,
        "01J00000000000000000000022",
        site_id,
        visitor_id,
        first + chrono::Duration::minutes(40),
        "/new-session",
    )
    .await;
    insert_identified_event(
        &pool,
        "01J00000000000000000000025",
        site_id,
        second_visitor_id,
        first + chrono::Duration::minutes(5),
        "/second-visitor",
    )
    .await;

    let original_payload = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT payload FROM raw_events WHERE site_id = $1 AND event_id = $2",
    )
    .bind(site_id)
    .bind("01J00000000000000000000020")
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(processor.process_all_once().await.unwrap(), 4);

    let generation_id = sqlx::query_scalar::<_, String>(
        "SELECT generation_id::text FROM analytics_generations
         WHERE site_id = $1 AND status = 'active'",
    )
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let visitor_facts = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM visitor_event_facts WHERE generation_id = $1::uuid",
    )
    .bind(&generation_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let session_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sessions WHERE generation_id = $1::uuid",
    )
    .bind(&generation_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let normalized_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM normalized_event_context WHERE generation_id = $1::uuid",
    )
    .bind(&generation_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let dimension_fact_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM dimension_event_facts WHERE generation_id = $1::uuid",
    )
    .bind(&generation_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(visitor_facts, 4);
    assert_eq!(session_count, 3);
    assert_eq!(normalized_count, 4);
    assert_eq!(dimension_fact_count, 24);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT page_views FROM dimension_daily
             WHERE generation_id = $1::uuid AND site_id = $2
               AND day = $3 AND dimension = 'browser' AND value = 'unknown'",
        )
        .bind(&generation_id)
        .bind(site_id)
        .bind(first.date_naive())
        .fetch_one(&pool)
        .await
        .unwrap(),
        3
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT unique_visitors FROM visitor_daily
             WHERE generation_id = $1::uuid AND site_id = $2 AND day = $3",
        )
        .bind(&generation_id)
        .bind(site_id)
        .bind(first.date_naive())
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );
    assert!(
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT source_watermark FROM analytics_generations
             WHERE generation_id = $1::uuid",
        )
        .bind(&generation_id)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("visitor_session")
        .is_some()
    );
    assert!(
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT source_watermark FROM analytics_generations
             WHERE generation_id = $1::uuid",
        )
        .bind(&generation_id)
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("dimensions")
        .is_some()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_watermarks
             WHERE site_id = $1 AND generation_id IS NULL AND source_name = 'page_views'",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );

    let payload_after = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT payload FROM raw_events WHERE site_id = $1 AND event_id = $2",
    )
    .bind(site_id)
    .bind("01J00000000000000000000020")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(payload_after, original_payload);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_generations
             WHERE site_id = $1 AND status = 'active'",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn analytics_disabled_keeps_page_view_workflow_without_rebuild_queue() {
    let (processor, pool) = setup().await;
    insert_raw_event(
        &pool,
        "01J00000000000000000000023",
        "site_phase6_disabled",
        "2026-09-18T12:00:00Z".parse().unwrap(),
        "/legacy",
    )
    .await;

    assert_eq!(processor.process_all_once().await.unwrap(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT page_views FROM page_view_totals WHERE site_id = $1",)
            .bind("site_phase6_disabled")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM analytics_rebuild_queue")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn no_visitor_event_contributes_dimensions_only_after_site_rebuild() {
    let (processor, pool) = setup().await;
    let site_id = "site_phase6_no_visitor";
    let occurred_at: DateTime<Utc> = "2026-09-18T12:00:00Z".parse().unwrap();
    sqlx::query(
        "INSERT INTO analytics_feature_flags (site_id, analytics_enabled)
         VALUES ($1, TRUE)",
    )
    .bind(site_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO raw_events
            (site_id, event_id, schema_version, event_type, occurred_at,
             received_at, path, payload, context_schema_version)
         VALUES ($1, $2, 1, 'page_view', $3, $3 + INTERVAL '1 minute', '/', $4, 1)",
    )
    .bind(site_id)
    .bind("01J00000000000000000000030")
    .bind(occurred_at)
    .bind(json!({
        "schema_version": 1,
        "event_id": "01J00000000000000000000030",
        "type": "page_view",
        "site_id": site_id,
        "occurred_at": occurred_at.timestamp_millis(),
        "path": "/",
        "context_schema_version": 1,
        "context": {
            "language": "en-CA",
            "timezone": "America/Toronto",
            "referrer": "https://example.com/previous",
            "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0"
        }
    }))
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(processor.process_all_once().await.unwrap(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_rebuild_queue WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );

    processor
        .rebuild_site(
            site_id,
            occurred_at.date_naive(),
            occurred_at.date_naive(),
            "initial",
            "woothee-0.13.0",
            false,
        )
        .await
        .unwrap();
    let generation_id = sqlx::query_scalar::<_, String>(
        "SELECT generation_id::text FROM analytics_generations
         WHERE site_id = $1 AND status = 'active'",
    )
    .bind(site_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM dimension_event_facts
             WHERE generation_id = $1::uuid AND site_id = $2",
        )
        .bind(&generation_id)
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        6
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM visitor_event_facts
             WHERE generation_id = $1::uuid AND site_id = $2",
        )
        .bind(&generation_id)
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM session_events
             WHERE generation_id = $1::uuid AND site_id = $2",
        )
        .bind(&generation_id)
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn disabled_analytics_pauses_pending_rebuild_without_activation() {
    let (processor, pool) = setup().await;
    let site_id = "site_paused_queue";
    let day = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();

    sqlx::query(
        "INSERT INTO analytics_feature_flags (site_id, analytics_enabled)
         VALUES ($1, FALSE)",
    )
    .bind(site_id)
    .execute(&pool)
    .await
    .unwrap();
    processor
        .enqueue_rebuild(site_id, day, day, "incremental", "woothee-0.13.0")
        .await
        .unwrap();

    assert!(!processor.process_rebuild_queue_once().await.unwrap());
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM analytics_rebuild_queue WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "pending"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_generations WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn rebuild_queue_deduplicates_same_site_scope() {
    let (processor, pool) = setup().await;
    let day = NaiveDate::from_ymd_opt(2026, 9, 18).unwrap();
    processor
        .enqueue_rebuild("site_queue", day, day, "incremental", "woothee-0.13.0")
        .await
        .unwrap();
    processor
        .enqueue_rebuild("site_queue", day, day, "backfill", "woothee-0.13.0")
        .await
        .unwrap();
    processor
        .enqueue_rebuild("site_queue", day, day, "incremental", "woothee-0.13.0")
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_rebuild_queue
             WHERE site_id = 'site_queue'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT rebuild_reason FROM analytics_rebuild_queue
             WHERE site_id = 'site_queue'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        "backfill"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn concurrent_queue_workers_publish_one_generation_without_failed_duplicate() {
    let (processor, pool) = setup().await;
    let second_processor = Processor::connect(&database_url()).await.unwrap();
    let site_id = "site_concurrent_queue";
    let visitor_id = "550e8400-e29b-41d4-a716-446655440000";
    let occurred_at: DateTime<Utc> = "2026-09-18T12:00:00Z".parse().unwrap();

    sqlx::query(
        "INSERT INTO analytics_feature_flags (site_id, analytics_enabled)
         VALUES ($1, TRUE)",
    )
    .bind(site_id)
    .execute(&pool)
    .await
    .unwrap();
    insert_identified_event(
        &pool,
        "01J00000000000000000000026",
        site_id,
        visitor_id,
        occurred_at,
        "/concurrent",
    )
    .await;

    assert!(processor.process_one().await.unwrap());
    let (left, right) = tokio::join!(
        processor.process_rebuild_queue_once(),
        second_processor.process_rebuild_queue_once()
    );
    let left = left.unwrap();
    let right = right.unwrap();
    assert!(left || right);

    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_generations
             WHERE site_id = $1 AND status = 'active'",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_generations
             WHERE site_id = $1 AND status = 'failed'",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM analytics_rebuild_queue WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "completed"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn late_events_over_24_hours_wait_for_explicit_backfill() {
    let (processor, pool) = setup().await;
    let site_id = "site_backfill";
    let visitor_id = "550e8400-e29b-41d4-a716-446655440000";
    sqlx::query(
        "INSERT INTO analytics_feature_flags (site_id, analytics_enabled)
         VALUES ($1, TRUE)",
    )
    .bind(site_id)
    .execute(&pool)
    .await
    .unwrap();
    insert_identified_event(
        &pool,
        "01J00000000000000000000024",
        site_id,
        visitor_id,
        "2026-09-18T12:00:00Z".parse().unwrap(),
        "/late",
    )
    .await;
    sqlx::query(
        "UPDATE raw_events
         SET received_at = occurred_at + INTERVAL '25 hours'
         WHERE site_id = $1",
    )
    .bind(site_id)
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(processor.process_all_once().await.unwrap(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT rebuild_reason FROM analytics_rebuild_queue WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "backfill"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM analytics_generations WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );

    processor
        .rebuild_site(
            site_id,
            NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
            "backfill",
            "woothee-0.13.0",
            false,
        )
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status FROM analytics_rebuild_queue WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "completed"
    );
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn processes_daily_routes_and_totals_atomically() {
    let (processor, pool) = setup().await;
    let day = NaiveDate::from_ymd_opt(2026, 9, 18)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    insert_raw_event(&pool, "01J00000000000000000000010", "site_a", day, "/").await;
    insert_raw_event(&pool, "01J00000000000000000000011", "site_a", day, "/about").await;
    insert_raw_event(&pool, "01J00000000000000000000012", "site_a", day, "/").await;

    assert_eq!(processor.process_all_once().await.unwrap(), 3);
    assert_eq!(processor.process_all_once().await.unwrap(), 0);

    let daily =
        sqlx::query("SELECT page_views FROM page_view_daily WHERE site_id = 'site_a' AND day = $1")
            .bind(day.date_naive())
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i64, _>("page_views");
    assert_eq!(daily, 3);

    let root = sqlx::query(
        "SELECT page_views FROM page_view_routes
         WHERE site_id = 'site_a' AND day = $1 AND path = '/'",
    )
    .bind(day.date_naive())
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<i64, _>("page_views");
    assert_eq!(root, 2);

    let total = sqlx::query("SELECT page_views FROM page_view_totals WHERE site_id = 'site_a'")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i64, _>("page_views");
    assert_eq!(total, 3);

    let processed =
        sqlx::query("SELECT COUNT(*) AS count FROM raw_events WHERE processed_at IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i64, _>("count");
    assert_eq!(processed, 3);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn uses_occurred_at_utc_day_and_isolates_sites() {
    let (processor, pool) = setup().await;
    let late_event = DateTime::parse_from_rfc3339("2026-09-18T00:30:00-04:00")
        .unwrap()
        .with_timezone(&Utc);
    insert_raw_event(
        &pool,
        "01J00000000000000000000013",
        "site_a",
        late_event,
        "/late",
    )
    .await;
    insert_raw_event(
        &pool,
        "01J00000000000000000000014",
        "site_b",
        late_event,
        "/late",
    )
    .await;

    assert_eq!(processor.process_all_once().await.unwrap(), 2);
    let rows = sqlx::query("SELECT site_id, day, page_views FROM page_view_daily ORDER BY site_id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String, _>("site_id"), "site_a");
    assert_eq!(rows[1].get::<String, _>("site_id"), "site_b");
    assert_eq!(
        rows[0].get::<NaiveDate, _>("day"),
        NaiveDate::from_ymd_opt(2026, 9, 18).unwrap()
    );
    assert_eq!(rows[0].get::<i64, _>("page_views"), 1);
    assert_eq!(rows[1].get::<i64, _>("page_views"), 1);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn failed_aggregate_update_leaves_event_unprocessed() {
    let (processor, pool) = setup().await;
    let day = Utc::now();
    insert_raw_event(
        &pool,
        "01J00000000000000000000015",
        "site_a",
        day,
        "/failure",
    )
    .await;
    sqlx::query("DROP TABLE page_view_routes")
        .execute(&pool)
        .await
        .unwrap();

    assert!(processor.process_one().await.is_err());
    let processed = sqlx::query(
        "SELECT processed_at FROM raw_events WHERE event_id = '01J00000000000000000000015'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<Option<DateTime<Utc>>, _>("processed_at");
    assert!(processed.is_none());

    for table in ["page_view_daily", "page_view_totals"] {
        let count = sqlx::query(&format!("SELECT COUNT(*) AS count FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i64, _>("count");
        assert_eq!(count, 0, "{table} should be rolled back");
    }

    sqlx::query(
        "CREATE TABLE page_view_routes (
            site_id VARCHAR(64) NOT NULL,
            day DATE NOT NULL,
            path TEXT NOT NULL,
            page_views BIGINT NOT NULL DEFAULT 0,
            PRIMARY KEY (site_id, day, path)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn once_cli_processes_the_backlog() {
    let (_processor, pool) = setup().await;
    let day = Utc::now();
    insert_raw_event(&pool, "01J00000000000000000000016", "site_cli", day, "/cli").await;

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_processor"))
        .env("DATABASE_URL", database_url())
        .arg("--once")
        .status()
        .expect("processor binary should start");
    assert!(status.success());

    let processed = sqlx::query(
        "SELECT processed_at FROM raw_events WHERE event_id = '01J00000000000000000000016'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<Option<DateTime<Utc>>, _>("processed_at");
    assert!(processed.is_some());
}
