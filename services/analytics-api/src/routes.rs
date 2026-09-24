use axum::{Router, routing::get};

use crate::{handlers, state::AppState};

pub(crate) fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health::health))
        .route(
            "/v1/sites/{site_id}/overview",
            get(handlers::overview::overview),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/overview",
            get(handlers::reports::range_overview),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/timeline",
            get(handlers::reports::timeline),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/pages",
            get(handlers::reports::pages),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/conversions",
            get(handlers::reports::conversions),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/funnels",
            get(handlers::reports::funnels),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/web-vitals",
            get(handlers::reports::web_vitals),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/events",
            get(handlers::reports::events),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/visitors",
            get(handlers::phase6::visitors),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/sessions",
            get(handlers::phase6::sessions),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/dimensions/{dimension}",
            get(handlers::phase6::dimensions),
        )
        .with_state(state)
}
