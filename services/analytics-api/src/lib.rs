use axum::{
    Json, Router,
    extract::{Path, RawQuery, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use url::form_urlencoded;

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;
const MAX_RANGE_DAYS: i64 = 365;
const INTERNAL_ERROR_MESSAGE: &str = "Analytics API failed to complete the request";

#[derive(Clone)]
pub struct AppState {
    pool: PgPool,
}

pub fn state(pool: PgPool) -> AppState {
    AppState { pool }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    site_id: String,
    page_views: i64,
}

#[derive(Debug, Serialize)]
struct RangeOverviewResponse {
    site_id: String,
    from: String,
    to: String,
    page_views: i64,
}

#[derive(Debug, Serialize)]
struct TimelineItem {
    day: NaiveDate,
    page_views: i64,
}

#[derive(Debug, Serialize)]
struct TimelineResponse {
    site_id: String,
    from: String,
    to: String,
    items: Vec<TimelineItem>,
}

#[derive(Debug, Serialize)]
struct PageItem {
    path: String,
    page_views: i64,
}

#[derive(Debug, Serialize)]
struct PagesResponse {
    site_id: String,
    from: String,
    to: String,
    items: Vec<PageItem>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug, Clone, Copy)]
struct DateRange {
    from: NaiveDate,
    to: NaiveDate,
}

#[derive(Debug)]
enum RequestError {
    InvalidDateRange(&'static str),
    DateRangeTooLarge,
    InvalidLimit,
}

impl IntoResponse for RequestError {
    fn into_response(self) -> axum::response::Response {
        let (code, message) = match self {
            Self::InvalidDateRange(message) => ("invalid_date_range", message),
            Self::DateRangeTooLarge => (
                "date_range_too_large",
                "date range must not exceed 366 days",
            ),
            Self::InvalidLimit => ("invalid_limit", "limit must be between 1 and 100"),
        };
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody { code, message },
            }),
        )
            .into_response()
    }
}

pub fn connect(database_url: &str) -> Result<AppState, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(database_url)?;
    Ok(state(pool))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/sites/{site_id}/overview", get(overview))
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/overview",
            get(range_overview),
        )
        .route(
            "/v1/sites/{site_id}/reports/{from}/{to}/timeline",
            get(timeline),
        )
        .route("/v1/sites/{site_id}/reports/{from}/{to}/pages", get(pages))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, HealthError> {
    sqlx::query("SELECT 1")
        .execute(&state.pool)
        .await
        .map_err(HealthError::database)?;
    Ok(Json(HealthResponse { status: "ok" }))
}

async fn overview(
    State(state): State<AppState>,
    Path(site_id): Path<String>,
) -> Result<Json<OverviewResponse>, ApiError> {
    let page_views = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE((SELECT page_views FROM page_view_totals WHERE site_id = $1), 0)",
    )
    .bind(&site_id)
    .fetch_one(&state.pool)
    .await
    .map_err(ApiError::database)?;
    Ok(Json(OverviewResponse {
        site_id,
        page_views,
    }))
}

async fn range_overview(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
) -> Result<axum::response::Response, HandlerError> {
    let range = parse_range(&from, &to)?;
    let page_views = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT SUM(page_views)::bigint FROM page_view_daily WHERE site_id = $1 AND day BETWEEN $2 AND $3",
    )
    .bind(&site_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_one(&state.pool)
    .await
    .map_err(ApiError::database)?
    .unwrap_or(0);
    Ok(Json(RangeOverviewResponse {
        site_id,
        from,
        to,
        page_views,
    })
    .into_response())
}

async fn timeline(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
) -> Result<axum::response::Response, HandlerError> {
    let range = parse_range(&from, &to)?;
    let rows = sqlx::query_as::<_, (NaiveDate, i64)>(
        "SELECT day, page_views FROM page_view_daily
         WHERE site_id = $1 AND day BETWEEN $2 AND $3 ORDER BY day ASC",
    )
    .bind(&site_id)
    .bind(range.from)
    .bind(range.to)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    let items = rows
        .into_iter()
        .map(|(day, page_views)| TimelineItem { day, page_views })
        .collect();
    Ok(Json(TimelineResponse {
        site_id,
        from,
        to,
        items,
    })
    .into_response())
}

async fn pages(
    State(state): State<AppState>,
    Path((site_id, from, to)): Path<(String, String, String)>,
    RawQuery(raw_query): RawQuery,
) -> Result<axum::response::Response, HandlerError> {
    let range = parse_range(&from, &to)?;
    let limit = parse_limit_query(raw_query.as_deref())?;
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT path, SUM(page_views)::bigint AS page_views
         FROM page_view_routes
         WHERE site_id = $1 AND day BETWEEN $2 AND $3
         GROUP BY path ORDER BY page_views DESC, path ASC LIMIT $4",
    )
    .bind(&site_id)
    .bind(range.from)
    .bind(range.to)
    .bind(limit)
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::database)?;
    let items = rows
        .into_iter()
        .map(|(path, page_views)| PageItem { path, page_views })
        .collect();
    Ok(Json(PagesResponse {
        site_id,
        from,
        to,
        items,
    })
    .into_response())
}

fn parse_range(from: &str, to: &str) -> Result<DateRange, RequestError> {
    let from_date = parse_date(from)?;
    let to_date = parse_date(to)?;
    if from_date > to_date {
        return Err(RequestError::InvalidDateRange(
            "from must be before or equal to to",
        ));
    }
    if to_date.signed_duration_since(from_date).num_days() > MAX_RANGE_DAYS {
        return Err(RequestError::DateRangeTooLarge);
    }
    Ok(DateRange {
        from: from_date,
        to: to_date,
    })
}

fn parse_date(value: &str) -> Result<NaiveDate, RequestError> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| RequestError::InvalidDateRange("from and to must use YYYY-MM-DD"))?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(RequestError::InvalidDateRange(
            "from and to must use YYYY-MM-DD",
        ));
    }
    Ok(date)
}

fn parse_limit(value: Option<&str>) -> Result<i64, RequestError> {
    let limit = value.map_or(Ok(DEFAULT_LIMIT), |value| {
        value.parse::<i64>().map_err(|_| RequestError::InvalidLimit)
    })?;
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(RequestError::InvalidLimit);
    }
    Ok(limit)
}

fn parse_limit_query(query: Option<&str>) -> Result<i64, RequestError> {
    let mut limit = None;
    for (key, value) in form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key == "limit" {
            if limit.is_some() {
                return Err(RequestError::InvalidLimit);
            }
            limit = Some(value.into_owned());
        }
    }
    parse_limit(limit.as_deref())
}

#[derive(Debug)]
struct ApiError;

#[derive(Debug)]
struct HealthError;

impl HealthError {
    fn database(error: sqlx::Error) -> Self {
        tracing::error!(%error, "analytics health database check failed");
        Self
    }
}

impl IntoResponse for HealthError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "analytics_api_error",
                    message: INTERNAL_ERROR_MESSAGE,
                },
            }),
        )
            .into_response()
    }
}

#[derive(Debug)]
enum HandlerError {
    Request(RequestError),
    Api(ApiError),
}

impl From<RequestError> for HandlerError {
    fn from(error: RequestError) -> Self {
        Self::Request(error)
    }
}

impl From<ApiError> for HandlerError {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

impl IntoResponse for HandlerError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Request(error) => error.into_response(),
            Self::Api(error) => error.into_response(),
        }
    }
}

impl ApiError {
    fn database(error: sqlx::Error) -> Self {
        tracing::error!(%error, "analytics database query failed");
        Self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "analytics_api_error",
                    message: INTERNAL_ERROR_MESSAGE,
                },
            }),
        )
            .into_response()
    }
}

pub fn parse_date_range_for_test(from: &str, to: &str) -> Result<(), &'static str> {
    parse_range(from, to)
        .map(|_| ())
        .map_err(|error| match error {
            RequestError::InvalidDateRange(_) => "invalid_date_range",
            RequestError::DateRangeTooLarge => "date_range_too_large",
            RequestError::InvalidLimit => "invalid_limit",
        })
}

pub fn parse_limit_for_test(value: Option<&str>) -> Result<i64, &'static str> {
    parse_limit(value).map_err(|_| "invalid_limit")
}

#[cfg(test)]
mod tests {
    use super::{parse_date_range_for_test, parse_limit_for_test};

    #[test]
    fn validates_inclusive_366_day_range() {
        assert!(parse_date_range_for_test("2026-01-01", "2027-01-01").is_ok());
        assert_eq!(
            parse_date_range_for_test("2026-01-01", "2027-01-02"),
            Err("date_range_too_large")
        );
    }

    #[test]
    fn rejects_invalid_date_ranges() {
        assert_eq!(
            parse_date_range_for_test("2026-02-30", "2026-03-01"),
            Err("invalid_date_range")
        );
        assert_eq!(
            parse_date_range_for_test("2026-03-02", "2026-03-01"),
            Err("invalid_date_range")
        );
    }

    #[test]
    fn validates_limit() {
        assert_eq!(parse_limit_for_test(None), Ok(20));
        assert_eq!(parse_limit_for_test(Some("100")), Ok(100));
        assert_eq!(parse_limit_for_test(Some("0")), Err("invalid_limit"));
        assert_eq!(parse_limit_for_test(Some("101")), Err("invalid_limit"));
        assert_eq!(parse_limit_for_test(Some("nope")), Err("invalid_limit"));
    }
}
