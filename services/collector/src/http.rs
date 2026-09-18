use axum::{Json, Router, routing::get};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, body::to_bytes, http::Request};
    use tower::ServiceExt;

    use super::router;

    #[tokio::test]
    async fn health_returns_ok_json() {
        let response = router()
            .oneshot(
                Request::get("/health")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("health request should complete");

        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "application/json");

        let body = to_bytes(response.into_body(), 1024)
            .await
            .expect("health body should be readable");
        assert_eq!(body.as_ref(), br#"{"status":"ok"}"#);
    }

    #[tokio::test]
    async fn events_route_is_not_registered_in_pr2() {
        let response = router()
            .oneshot(
                Request::post("/v1/events")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should complete");

        assert_eq!(response.status(), 404);
    }
}
