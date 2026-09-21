use axum::{body::Body, http::Request};
use chrono::Utc;
use collector::{
    config::{SiteConfig, SiteRegistry},
    feature_flags::FeatureFlagStore,
    http::router,
    protocol::{EventType, PageViewEvent},
    rate_limit::RateLimiter,
    security::KeyPolicy,
    sink::{EventSink, PostgresSink, SinkError, StoredEvent},
    validation::Validator,
};
use http_body_util::BodyExt;
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use tower::ServiceExt;

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point to the integration PostgreSQL database")
}

async fn setup() -> (PostgresSink, PgPool) {
    let url = database_url();
    PostgresSink::migrate(&url)
        .await
        .expect("database migrations should succeed");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
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
    .expect("integration tables should be writable");
    sqlx::query("DELETE FROM analytics_feature_flags")
        .execute(&pool)
        .await
        .expect("feature flag table should be writable");
    let sink = PostgresSink::connect(&url)
        .await
        .expect("Postgres sink should connect");
    (sink, pool)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn stores_v2_metadata_and_reads_site_feature_flags() {
    let (sink, pool) = setup().await;
    let visitor_id = "550e8400-e29b-41d4-a716-446655440000";
    let mut event = stored_event(
        "site_example",
        "01J00000000000000000000004",
        json!({
            "schema_version": 2,
            "event_id": "01J00000000000000000000004",
            "type": "page_view",
            "site_id": "site_example",
            "occurred_at": 1760000000000_i64,
            "path": "/v2",
            "visitor_id": visitor_id,
            "context_schema_version": 1
        }),
    );
    event.event.schema_version = 2;
    event.event.visitor_id = Some(visitor_id.to_owned());
    event.event.context_schema_version = Some(1);

    assert!(
        !sink
            .protocol_v2_enabled("site_example")
            .await
            .expect("missing flag should be disabled")
    );
    sqlx::query(
        "INSERT INTO analytics_feature_flags (site_id, protocol_v2_enabled)
         VALUES ($1, TRUE)
         ON CONFLICT (site_id) DO UPDATE SET protocol_v2_enabled = EXCLUDED.protocol_v2_enabled",
    )
    .bind("site_example")
    .execute(&pool)
    .await
    .expect("feature flag should be writable");
    assert!(
        sink.protocol_v2_enabled("site_example")
            .await
            .expect("feature flag should be readable")
    );

    sink.accept(vec![event])
        .await
        .expect("V2 event should insert");
    let row = sqlx::query(
        "SELECT visitor_id::text AS visitor_id, context_schema_version, payload
         FROM raw_events WHERE event_id = $1",
    )
    .bind("01J00000000000000000000004")
    .fetch_one(&pool)
    .await
    .expect("V2 raw event should be queryable");
    assert_eq!(row.get::<String, _>("visitor_id"), visitor_id);
    assert_eq!(row.get::<i32, _>("context_schema_version"), 1);
    assert_eq!(
        row.get::<serde_json::Value, _>("payload")["schema_version"],
        2
    );
}

fn stored_event(site_id: &str, event_id: &str, payload: serde_json::Value) -> StoredEvent {
    StoredEvent {
        event: PageViewEvent {
            schema_version: 1,
            event_id: event_id.to_owned(),
            event_type: EventType::PageView,
            site_id: site_id.to_owned(),
            occurred_at: 1_760_000_000_000,
            path: "/about".to_owned(),
            url: Some("https://example.com/about".to_owned()),
            title: Some("About".to_owned()),
            referrer: None,
            context: None,
            visitor_id: None,
            context_schema_version: None,
        },
        payload,
        received_at: Utc::now(),
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn stores_raw_payload_and_applies_site_event_idempotency() {
    let (sink, pool) = setup().await;
    let payload = json!({
        "schema_version": 1,
        "event_id": "01J00000000000000000000000",
        "type": "page_view",
        "site_id": "site_example",
        "occurred_at": 1760000000000_i64,
        "path": "/about",
        "future_field": {"kept": true}
    });
    let duplicate = stored_event(
        "site_example",
        "01J00000000000000000000000",
        payload.clone(),
    );

    sink.accept(vec![duplicate.clone()])
        .await
        .expect("first insert should succeed");
    sink.accept(vec![duplicate])
        .await
        .expect("duplicate insert should be a successful no-op");
    sink.accept(vec![stored_event(
        "other_site",
        "01J00000000000000000000000",
        json!({"site_id": "other_site"}),
    )])
    .await
    .expect("same event id on another site should succeed");

    let rows = sqlx::query(
        "SELECT site_id, payload, processed_at, occurred_at, received_at, created_at
         FROM raw_events ORDER BY site_id",
    )
    .fetch_all(&pool)
    .await
    .expect("raw events should be queryable");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String, _>("site_id"), "other_site");
    assert_eq!(rows[1].get::<String, _>("site_id"), "site_example");
    assert_eq!(rows[1].get::<serde_json::Value, _>("payload"), payload);
    assert!(
        rows[1]
            .get::<Option<chrono::DateTime<Utc>>, _>("processed_at")
            .is_none()
    );
    assert!(
        rows[1]
            .get::<chrono::DateTime<Utc>, _>("occurred_at")
            .timestamp()
            > 0
    );
    assert!(rows[1].get::<chrono::DateTime<Utc>, _>("received_at") <= Utc::now());
    assert!(rows[1].get::<chrono::DateTime<Utc>, _>("created_at") <= Utc::now());

    let total_count = sqlx::query("SELECT COUNT(*) AS count FROM page_view_totals")
        .fetch_one(&pool)
        .await
        .expect("totals table should be queryable")
        .get::<i64, _>("count");
    assert_eq!(total_count, 0);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn batch_rolls_back_when_a_later_event_cannot_be_stored() {
    let (sink, pool) = setup().await;
    let mut invalid = stored_event(
        "site_example",
        "01J00000000000000000000001",
        json!({"event_id": "01J00000000000000000000001"}),
    );
    invalid.event.occurred_at = i64::MAX;

    let result = sink
        .accept(vec![
            stored_event(
                "site_example",
                "01J00000000000000000000002",
                json!({"event_id": "01J00000000000000000000002"}),
            ),
            invalid,
        ])
        .await;
    assert!(matches!(result, Err(SinkError::InvalidTimestamp)));

    let count = sqlx::query("SELECT COUNT(*) AS count FROM raw_events")
        .fetch_one(&pool)
        .await
        .expect("raw events should be queryable")
        .get::<i64, _>("count");
    assert_eq!(count, 0);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn collector_http_writes_to_postgres_sink() {
    let (sink, pool) = setup().await;
    let site = SiteConfig {
        site_id: "site_example".to_owned(),
        environment: "production".to_owned(),
        enabled: true,
        allowed_origins: vec!["https://example.com".to_owned()],
        ingest_keys: vec!["production-key".to_owned()],
    };
    let policy = KeyPolicy::new(
        SiteRegistry::from_sites(vec![site]).expect("test site config should be valid"),
    );
    let app = router(
        Validator::new().expect("event schemas should compile"),
        sink,
        policy,
        RateLimiter::new(),
    );
    let request = Request::builder()
        .method("POST")
        .uri("/v1/events")
        .header("content-type", "application/json")
        .header("origin", "https://example.com")
        .header("x-ingest-key", "production-key")
        .body(Body::from(
            json!({
                "schema_version": 1,
                "events": [{
                    "schema_version": 1,
                    "event_id": "01J00000000000000000000003",
                    "type": "page_view",
                    "site_id": "site_example",
                    "occurred_at": 1760000000000_i64,
                    "path": "/from-http",
                    "future_field": "preserved"
                }]
            })
            .to_string(),
        ))
        .expect("request should build");

    let response = app.oneshot(request).await.expect("request should complete");
    assert_eq!(response.status(), 202);
    let _ = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable");

    let row = sqlx::query("SELECT payload FROM raw_events WHERE event_id = $1")
        .bind("01J00000000000000000000003")
        .fetch_one(&pool)
        .await
        .expect("HTTP event should be stored");
    assert_eq!(
        row.get::<serde_json::Value, _>("payload")["future_field"],
        "preserved"
    );
}
