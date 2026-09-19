use axum::{Json, extract::State};

use crate::{errors::HealthError, models::HealthResponse, queries, state::AppState};

pub(crate) async fn health(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, HealthError> {
    queries::health(&state.pool)
        .await
        .map_err(HealthError::database)?;
    Ok(Json(HealthResponse { status: "ok" }))
}
