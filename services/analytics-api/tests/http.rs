use analytics_api::{router, state};
use axum::{body::to_bytes, http::Request};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;
use tower::ServiceExt;

async fn pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(2)
        .connect(&env::var("DATABASE_URL").expect("DATABASE_URL is required"))
        .await
        .expect("database should be available")
}

async fn body(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body should be readable");
    serde_json::from_slice(&bytes).expect("response should be JSON")
}

async fn reset(pool: &PgPool, site_id: &str) {
    for table in ["page_view_routes", "page_view_daily", "page_view_totals"] {
        sqlx::query(&format!("DELETE FROM {table} WHERE site_id = $1"))
            .bind(site_id)
            .execute(pool)
            .await
            .unwrap();
    }
}

fn app(pool: PgPool) -> axum::Router {
    router(state(pool))
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and PostgreSQL"]
async fn analytics_http_contract_returns_aggregates_and_validates_queries() {
    let pool = pool().await;
    let site = "analytics_api_test";
    reset(&pool, site).await;
    sqlx::query("INSERT INTO page_view_totals (site_id, page_views) VALUES ($1, 9)")
        .bind(site)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO page_view_daily (site_id, day, page_views) VALUES ($1, '2026-09-18', 3), ($1, '2026-09-19', 2)")
        .bind(site).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO page_view_routes (site_id, day, path, page_views) VALUES ($1, '2026-09-18', '/', 2), ($1, '2026-09-18', '/about', 1), ($1, '2026-09-19', '/about', 2)")
        .bind(site).execute(&pool).await.unwrap();

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!("/v1/sites/{site}/overview"))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["page_views"], 9);

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-18/2026-09-19/overview"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["page_views"], 5);

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-18/2026-09-19/timeline"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        body(response).await["items"],
        serde_json::json!([
            {"day":"2026-09-18", "page_views":3},
            {"day":"2026-09-19", "page_views":2}
        ])
    );

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-18/2026-09-19/pages?limit=2"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        body(response).await["items"],
        serde_json::json!([
            {"path":"/about", "page_views":3}, {"path":"/", "page_views":2}
        ])
    );

    sqlx::query("INSERT INTO page_view_routes (site_id, day, path, page_views) VALUES ($1, '2026-09-18', '/contact', 2)")
        .bind(site)
        .execute(&pool)
        .await
        .unwrap();
    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-18/2026-09-19/pages"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        body(response).await["items"],
        serde_json::json!([
            {"path":"/about", "page_views":3},
            {"path":"/", "page_views":2},
            {"path":"/contact", "page_views":2}
        ])
    );

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-19/2026-09-18/overview"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(body(response).await["error"]["code"], "invalid_date_range");

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2025-01-01/2026-01-02/overview"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(
        body(response).await["error"]["code"],
        "date_range_too_large"
    );

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-18/2026-09-19/pages?limit=0"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(body(response).await["error"]["code"], "invalid_limit");

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-18/2026-09-19/pages?limit=1&limit=2"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(body(response).await["error"]["code"], "invalid_limit");

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-02-30/2026-03-01/overview"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 400);
    assert_eq!(body(response).await["error"]["code"], "invalid_date_range");

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2026-09-18/2026-09-18/overview"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["page_views"], 3);

    let response = app(pool.clone())
        .oneshot(
            Request::get(format!(
                "/v1/sites/{site}/reports/2025-01-01/2025-01-02/pages"
            ))
            .body(axum::body::Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["items"], serde_json::json!([]));

    let response = app(pool.clone())
        .oneshot(
            Request::get("/v1/sites/site_missing/reports/2026-09-18/2026-09-19/timeline")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["items"], serde_json::json!([]));
    let response = app(pool.clone())
        .oneshot(
            Request::get("/v1/sites/site_missing/overview")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await["page_views"], 0);
    reset(&pool, site).await;
}

#[tokio::test]
#[ignore = "requires DATABASE_URL and PostgreSQL"]
async fn health_checks_database() {
    let pool = pool().await;
    let response = app(pool)
        .oneshot(
            Request::get("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(body(response).await, serde_json::json!({"status":"ok"}));
}

#[tokio::test]
async fn health_hides_database_failure() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(100))
        .connect_lazy("postgres://invalid:invalid@127.0.0.1:1/invalid")
        .expect("lazy pool should be created");
    let response = app(pool)
        .oneshot(
            Request::get("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 503);
    let value = body(response).await;
    assert_eq!(value["error"]["code"], "analytics_api_error");
    assert_eq!(
        value["error"]["message"],
        "Analytics API failed to complete the request"
    );
}

#[tokio::test]
async fn analytics_errors_are_generic() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(100))
        .connect_lazy("postgres://invalid:invalid@127.0.0.1:1/invalid")
        .expect("lazy pool should be created");
    let response = app(pool)
        .oneshot(
            Request::get("/v1/sites/site_example/overview")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 500);
    let value = body(response).await;
    assert_eq!(value["error"]["code"], "analytics_api_error");
    assert_eq!(
        value["error"]["message"],
        "Analytics API failed to complete the request"
    );
}
