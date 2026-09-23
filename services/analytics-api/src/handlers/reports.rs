use axum::{
    Json,
    extract::{Path, RawQuery, State},
    response::{IntoResponse, Response},
};

use crate::{
    errors::{ApiError, HandlerError},
    models::{
        EventDailyItem, EventsReportResponse, PageItem, PagesResponse, RangeOverviewResponse,
        TimelineItem, TimelineResponse, WebVitalReportItem, WebVitalReportResponse,
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

pub(crate) async fn web_vitals(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
    RawQuery(raw_query): RawQuery,
) -> Result<Response, HandlerError> {
    let range = validation::parse_range(&from, &to)?;
    let (limit, path) = validation::parse_web_vitals_query(raw_query.as_deref())?;
    let rows = queries::web_vital_rows(&state.pool, &site_id, range, path.as_deref(), limit)
        .await
        .map_err(ApiError::database)?;
    let total = queries::web_vital_total(&state.pool, &site_id, range, path.as_deref())
        .await
        .map_err(ApiError::database)?;
    let items = rows
        .into_iter()
        .map(|row| WebVitalReportItem {
            path: row.path,
            metric: row.metric,
            count: row.count,
            p75: row.p75,
            good_count: row.good_count,
            needs_improvement_count: row.needs_improvement_count,
            poor_count: row.poor_count,
            status: if row.count < 4 {
                "insufficient_data"
            } else {
                "available"
            }
            .to_owned(),
        })
        .collect();
    let data_as_of = queries::web_vital_watermark(&state.pool, &site_id)
        .await
        .map_err(ApiError::database)?;
    let freshness_status = queries::web_vital_freshness(&state.pool, &site_id)
        .await
        .map_err(ApiError::database)?;
    Ok(Json(WebVitalReportResponse {
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
