use async_trait::async_trait;
use axum::{body::Body, body::to_bytes, http::Request};
use chrono::{Duration, Utc};
use collector::{
    config::CollectorConfig,
    feature_flags::{FeatureFlagError, FeatureFlagStore, StaticFeatureFlagStore},
    http::{router, router_with_feature_flags},
    rate_limit::RateLimiter,
    security::KeyPolicy,
    sink::{EventSink, InMemorySink},
    validation::Validator,
};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tower::ServiceExt;

const VALID_EVENT: &str = r#"{"schema_version":1,"event_id":"01J00000000000000000000000","type":"page_view","site_id":"site_example","occurred_at":1760000000000,"path":"/about"}"#;

fn app<S>(sink: S) -> axum::Router
where
    S: EventSink + 'static,
{
    app_with_limiter(sink, RateLimiter::new())
}

fn app_with_limiter<S>(sink: S, rate_limiter: RateLimiter) -> axum::Router
where
    S: EventSink + 'static,
{
    let config_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collector.pr5.toml");
    let config = CollectorConfig::load_from_path(&config_path).expect("test config should load");
    router(
        Validator::new().expect("schemas should compile"),
        sink,
        KeyPolicy::new(config.registry().expect("test registry should build")),
        rate_limiter,
    )
}

fn request(body: &str) -> Request<Body> {
    Request::post("/v1/events")
        .header("content-type", "application/json")
        .header("origin", "https://example.com")
        .header("x-ingest-key", "pr5-production-key")
        .body(Body::from(body.to_owned()))
        .expect("request should build")
}

#[derive(Clone)]
struct CountingFeatureFlags {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl FeatureFlagStore for CountingFeatureFlags {
    async fn protocol_v2_enabled(&self, _site_id: &str) -> Result<bool, FeatureFlagError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
}

struct FailingFeatureFlags;

#[async_trait]
impl FeatureFlagStore for FailingFeatureFlags {
    async fn protocol_v2_enabled(&self, _site_id: &str) -> Result<bool, FeatureFlagError> {
        Err(FeatureFlagError::Database(sqlx::Error::RowNotFound))
    }
}

fn app_with_flags<S, F>(sink: S, feature_flags: F) -> axum::Router
where
    S: EventSink + 'static,
    F: FeatureFlagStore + 'static,
{
    let config_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collector.pr5.toml");
    let config = CollectorConfig::load_from_path(&config_path).expect("test config should load");
    router_with_feature_flags(
        Validator::new().expect("schemas should compile"),
        sink,
        KeyPolicy::new(config.registry().expect("test registry should build")),
        RateLimiter::new(),
        feature_flags,
    )
}

fn enabled_app(sink: InMemorySink) -> axum::Router {
    let config_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collector.pr5.toml");
    let config = CollectorConfig::load_from_path(&config_path).expect("test config should load");
    router_with_feature_flags(
        Validator::new().expect("schemas should compile"),
        sink,
        KeyPolicy::new(config.registry().expect("test registry should build")),
        RateLimiter::new(),
        StaticFeatureFlagStore::enabled(),
    )
}

#[tokio::test]
async fn external_request_test_accepts_enabled_v2_without_visitor_id() {
    let sink = InMemorySink::new();
    let response = enabled_app(sink.clone())
        .oneshot(request(
            r#"{"schema_version":2,"events":[{"schema_version":2,"event_id":"01J00000000000000000000001","type":"page_view","site_id":"site_example","occurred_at":1760000000000,"path":"/v2"}]}"#,
        ))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 202);
    let stored = sink.snapshot().await;
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].schema_version, 2);
    assert_eq!(stored[0].visitor_id, None);
}

#[tokio::test]
async fn external_request_test_rejects_v2_when_flag_is_disabled() {
    let response = app(InMemorySink::new())
        .oneshot(request(
            r#"{"schema_version":2,"events":[{"schema_version":2,"event_id":"01J00000000000000000000007","type":"page_view","site_id":"site_example","occurred_at":1760000000000,"path":"/v2"}]}"#,
        ))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 400);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "invalid_event_batch"
    );
}

#[tokio::test]
async fn external_request_test_does_not_query_feature_flags_for_v1() {
    let calls = Arc::new(AtomicUsize::new(0));
    let response = app_with_flags(
        InMemorySink::new(),
        CountingFeatureFlags {
            calls: calls.clone(),
        },
    )
    .oneshot(request(&format!(
        r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
    )))
    .await
    .expect("request should complete");

    assert_eq!(response.status(), 202);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn external_request_test_returns_generic_error_when_v2_flag_lookup_fails() {
    let response = app_with_flags(InMemorySink::new(), FailingFeatureFlags)
        .oneshot(request(
            r#"{"schema_version":2,"events":[{"schema_version":2,"type":"page_view","site_id":"site_example","occurred_at":1760000000000,"path":"/v2"}]}"#,
        ))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 500);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "collector_error"
    );
}

#[tokio::test]
async fn external_request_test_rejects_v2_events_more_than_five_minutes_ahead() {
    let occurred_at = (Utc::now() + Duration::minutes(6)).timestamp_millis();
    let body = serde_json::json!({
        "schema_version": 2,
        "events": [{
            "schema_version": 2,
            "event_id": "01J00000000000000000000005",
            "type": "page_view",
            "site_id": "site_example",
            "occurred_at": occurred_at,
            "path": "/future"
        }]
    });
    let response = enabled_app(InMemorySink::new())
        .oneshot(request(&body.to_string()))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 400);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "invalid_occurred_at"
    );
}

#[tokio::test]
async fn external_request_test_accepts_v2_events_within_five_minutes() {
    let occurred_at = (Utc::now() + Duration::minutes(4)).timestamp_millis();
    let body = serde_json::json!({
        "schema_version": 2,
        "events": [{
            "schema_version": 2,
            "event_id": "01J00000000000000000000006",
            "type": "page_view",
            "site_id": "site_example",
            "occurred_at": occurred_at,
            "path": "/future"
        }]
    });
    let response = enabled_app(InMemorySink::new())
        .oneshot(request(&body.to_string()))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 202);
}

async fn response_json(response: axum::response::Response) -> serde_json::Value {
    let body = to_bytes(response.into_body(), 128 * 1024)
        .await
        .expect("response body should be readable");
    serde_json::from_slice(&body).expect("response body should be JSON")
}

#[tokio::test]
async fn external_request_test_accepts_batch_and_exposes_sink_snapshot() {
    let sink = InMemorySink::new();
    let response = app(sink.clone())
        .oneshot(request(&format!(
            r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
        )))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 202);
    assert_eq!(
        response_json(response).await,
        serde_json::json!({"accepted": 1})
    );
    assert_eq!(sink.snapshot().await.len(), 1);
}

#[tokio::test]
async fn external_request_test_rejects_invalid_batch_atomically() {
    let sink = InMemorySink::new();
    let body = format!(
        r#"{{"schema_version":1,"events":[{VALID_EVENT},{{"schema_version":1,"event_id":"bad","type":"page_view","site_id":"site_example","occurred_at":-1,"path":"/about"}}]}}"#
    );
    let response = app(sink.clone())
        .oneshot(request(&body))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 400);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "invalid_event_batch"
    );
    assert!(sink.snapshot().await.is_empty());
}

#[tokio::test]
async fn external_request_test_rejects_missing_key() {
    let response = app(InMemorySink::new())
        .oneshot(
            Request::post("/v1/events")
                .header("content-type", "application/json")
                .header("origin", "https://example.com")
                .body(Body::from(format!(
                    r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 401);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "invalid_ingest_key"
    );
}

#[tokio::test]
async fn external_request_test_rejects_unknown_site() {
    let response = app(InMemorySink::new())
        .oneshot(request(&format!(
            r#"{{"schema_version":1,"events":[{}]}}"#,
            VALID_EVENT.replace("site_example", "site_unknown")
        )))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 403);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "site_not_allowed"
    );
}

#[tokio::test]
async fn external_request_test_rejects_disabled_site() {
    let response = app(InMemorySink::new())
        .oneshot(
            Request::post("/v1/events")
                .header("content-type", "application/json")
                .header("origin", "https://disabled.example")
                .header("x-ingest-key", "pr5-disabled-key")
                .body(Body::from(format!(
                    r#"{{"schema_version":1,"events":[{}]}}"#,
                    VALID_EVENT.replace("site_example", "site_disabled")
                )))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 403);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "site_not_allowed"
    );
}

#[tokio::test]
async fn external_request_test_rejects_wrong_key() {
    let response = app(InMemorySink::new())
        .oneshot(
            Request::post("/v1/events")
                .header("content-type", "application/json")
                .header("origin", "https://example.com")
                .header("x-ingest-key", "not-a-configured-key")
                .body(Body::from(format!(
                    r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 401);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "invalid_ingest_key"
    );
}

#[tokio::test]
async fn external_request_test_returns_cors_headers_for_allowed_origin() {
    let response = app(InMemorySink::new())
        .oneshot(request(&format!(
            r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
        )))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 202);
    assert_eq!(
        response.headers()["access-control-allow-origin"],
        "https://example.com"
    );
    assert_eq!(response.headers()["vary"], "Origin");
}

#[tokio::test]
async fn external_request_test_rejects_missing_and_disallowed_origin() {
    let missing_origin = Request::post("/v1/events")
        .header("content-type", "application/json")
        .header("x-ingest-key", "pr5-production-key")
        .body(Body::from(format!(
            r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
        )))
        .expect("request should build");
    let response = app(InMemorySink::new())
        .oneshot(missing_origin)
        .await
        .expect("request should complete");
    assert_eq!(response.status(), 403);
    assert_eq!(
        response_json(response).await["error"]["code"],
        "origin_not_allowed"
    );

    let response = app(InMemorySink::new())
        .oneshot(
            Request::post("/v1/events")
                .header("content-type", "application/json")
                .header("origin", "https://evil.example")
                .header("x-ingest-key", "pr5-production-key")
                .body(Body::from(format!(
                    r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(response.status(), 403);
    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
    assert_eq!(response.headers()["vary"], "Origin");
    assert_eq!(
        response_json(response).await["error"]["code"],
        "origin_not_allowed"
    );
}

#[tokio::test]
async fn external_request_test_handles_cors_preflight() {
    let response = app(InMemorySink::new())
        .oneshot(
            Request::options("/v1/events")
                .header("origin", "https://example.com")
                .header("access-control-request-method", "POST")
                .header(
                    "access-control-request-headers",
                    "content-type, x-ingest-key",
                )
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 204);
    assert_eq!(
        response.headers()["access-control-allow-origin"],
        "https://example.com"
    );
    assert_eq!(response.headers()["access-control-allow-methods"], "POST");
    assert_eq!(
        response.headers()["access-control-allow-headers"],
        "Content-Type, X-Ingest-Key"
    );
    assert_eq!(response.headers()["access-control-max-age"], "600");
    assert_eq!(response.headers()["vary"], "Origin");
    assert!(
        to_bytes(response.into_body(), 1024)
            .await
            .expect("body should be readable")
            .is_empty()
    );

    let response = app(InMemorySink::new())
        .oneshot(
            Request::options("/v1/events")
                .header("origin", "https://evil.example")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should complete");
    assert_eq!(response.status(), 403);
    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );
    assert_eq!(response.headers()["vary"], "Origin");
}

#[tokio::test]
async fn external_request_test_accepts_localhost_development_origin() {
    let response = app(InMemorySink::new())
        .oneshot(
            Request::post("/v1/events")
                .header("content-type", "application/json")
                .header("origin", "http://localhost:3000")
                .header("x-ingest-key", "pr5-development-key")
                .body(Body::from(format!(
                    r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#
                )))
                .expect("request should build"),
        )
        .await
        .expect("request should complete");

    assert_eq!(response.status(), 202);
    assert_eq!(
        response.headers()["access-control-allow-origin"],
        "http://localhost:3000"
    );
}

#[tokio::test]
async fn external_request_test_rate_limits_at_production_boundary() {
    let sink = InMemorySink::new();
    let app = app_with_limiter(sink.clone(), RateLimiter::new());
    let body = format!(r#"{{"schema_version":1,"events":[{VALID_EVENT}]}}"#);

    for _ in 0..600 {
        let response = app
            .clone()
            .oneshot(request(&body))
            .await
            .expect("request should complete");
        assert_eq!(response.status(), 202);
    }

    let response = app
        .oneshot(request(&body))
        .await
        .expect("request should complete");
    assert_eq!(response.status(), 429);
    assert_eq!(response.headers()["retry-after"], "60");
    assert_eq!(
        response_json(response).await["error"]["code"],
        "rate_limited"
    );
    assert_eq!(sink.snapshot().await.len(), 600);
}
