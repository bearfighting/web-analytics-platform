use axum::{
    Json,
    extract::{Path, State},
    response::{IntoResponse, Response},
};

use crate::{
    errors::{ApiError, HandlerError},
    models::{GeoCountryItem, GeoCountryReportResponse},
    queries,
    state::AppState,
    validation,
};

pub(crate) async fn countries(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
) -> Result<Response, HandlerError> {
    let range = validation::parse_range(&from, &to)?;
    let items = queries::geo_country_rows(&state.pool, &site_id, range)
        .await
        .map_err(ApiError::database)?
        .into_iter()
        .map(|row| GeoCountryItem {
            country_code: row.country_code,
            page_views: row.page_views,
        })
        .collect();
    let coverage_from = queries::geo_country_coverage_from(&state.pool, &site_id)
        .await
        .map_err(ApiError::database)?;
    Ok(Json(GeoCountryReportResponse {
        site_id,
        from,
        to,
        coverage_from,
        items,
    })
    .into_response())
}
