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
        .with_state(state)
}
