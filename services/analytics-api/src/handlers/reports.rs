use axum::{
    Json,
    extract::{Path, RawQuery, State},
    response::{IntoResponse, Response},
};

use crate::{
    errors::{ApiError, HandlerError},
    models::{
        EventDailyItem, EventsReportResponse, PageItem, PagesResponse, RangeOverviewResponse,
        TimelineItem, TimelineResponse,
    },
    queries,
    state::AppState,
    validation,
};

pub(crate) async fn range_overview(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
) -> Result<Response, HandlerError> {
    let range = validation::parse_range(&from, &to)?;
    let page_views = queries::range_overview(&state.pool, &site_id, range)
        .await
        .map_err(ApiError::database)?;
    Ok(Json(RangeOverviewResponse {
        site_id,
        from,
        to,
        page_views,
    })
    .into_response())
}

pub(crate) async fn timeline(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
) -> Result<Response, HandlerError> {
    let range = validation::parse_range(&from, &to)?;
    let rows = queries::timeline(&state.pool, &site_id, range)
        .await
        .map_err(ApiError::database)?;
    let items = rows
        .into_iter()
        .map(|row| TimelineItem {
            day: row.day,
            page_views: row.page_views,
        })
        .collect();
    Ok(Json(TimelineResponse {
        site_id,
        from,
        to,
        items,
    })
    .into_response())
}

pub(crate) async fn pages(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
    RawQuery(raw_query): RawQuery,
) -> Result<Response, HandlerError> {
    let range = validation::parse_range(&from, &to)?;
    let limit = validation::parse_limit_query(raw_query.as_deref())?;
    let rows = queries::pages(&state.pool, &site_id, range, limit)
        .await
        .map_err(ApiError::database)?;
    let items = rows
        .into_iter()
        .map(|row| PageItem {
            path: row.path,
            page_views: row.page_views,
        })
        .collect();
    Ok(Json(PagesResponse {
        site_id,
        from,
        to,
        items,
    })
    .into_response())
}

pub(crate) async fn events(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
    RawQuery(raw_query): RawQuery,
) -> Result<Response, HandlerError> {
    let range = validation::parse_range(&from, &to)?;
    let (limit, event_name) = validation::parse_events_query(raw_query.as_deref())?;
    let total = queries::custom_event_total(&state.pool, &site_id, range, event_name.as_deref())
        .await
        .map_err(ApiError::database)?;
    let rows = queries::custom_event_daily_rows(
        &state.pool,
        &site_id,
        range,
        event_name.as_deref(),
        limit,
    )
    .await
    .map_err(ApiError::database)?;
    let items = rows
        .into_iter()
        .map(|row| EventDailyItem {
            day: row.day,
            event_name: row.event_name,
            event_count: row.event_count,
        })
        .collect();
    let data_as_of = queries::custom_event_watermark(&state.pool, &site_id)
        .await
        .map_err(ApiError::database)?;
    let freshness_status = queries::custom_event_freshness(&state.pool, &site_id)
        .await
        .map_err(ApiError::database)?;
    Ok(Json(EventsReportResponse {
        site_id,
        from,
        to,
        total,
        items,
        data_as_of,
        freshness_status,
        aggregation_version: 1,
    })
    .into_response())
}
