use axum::{body::Body, body::to_bytes, http::Request};
use collector::{
    config::CollectorConfig,
    http::router,
    security::KeyPolicy,
    sink::{EventSink, InMemorySink},
    validation::Validator,
};
use std::path::Path;
use tower::ServiceExt;

const VALID_EVENT: &str = r#"{"schema_version":1,"event_id":"01J00000000000000000000000","type":"page_view","site_id":"site_example","occurred_at":1760000000000,"path":"/about"}"#;

fn app<S>(sink: S) -> axum::Router
where
    S: EventSink + 'static,
{
    let config_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/collector.pr4.toml");
    let config = CollectorConfig::load_from_path(&config_path).expect("test config should load");
    router(
        Validator::new().expect("schemas should compile"),
        sink,
        KeyPolicy::new(config.registry().expect("test registry should build")),
    )
}

fn request(body: &str) -> Request<Body> {
    Request::post("/v1/events")
        .header("content-type", "application/json")
        .header("x-ingest-key", "test-production-key")
        .body(Body::from(body.to_owned()))
        .expect("request should build")
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
        .oneshot(request(&format!(
            r#"{{"schema_version":1,"events":[{}]}}"#,
            VALID_EVENT.replace("site_example", "site_disabled")
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
async fn external_request_test_rejects_wrong_key() {
    let response = app(InMemorySink::new())
        .oneshot(
            Request::post("/v1/events")
                .header("content-type", "application/json")
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
