use axum::{
    Json,
    extract::{Path, State},
};

use crate::{errors::ApiError, models::OverviewResponse, queries, state::AppState};

pub(crate) async fn overview(
    State(state): State<AppState>,
    Path(site_id): Path<String>,
) -> Result<Json<OverviewResponse>, ApiError> {
    let page_views = queries::overview(&state.pool, &site_id)
        .await
        .map_err(ApiError::database)?;
    Ok(Json(OverviewResponse {
        site_id,
        page_views,
    }))
}
