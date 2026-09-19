use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

const INTERNAL_ERROR_MESSAGE: &str = "Analytics API failed to complete the request";

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

#[derive(Debug)]
pub(crate) enum RequestError {
    InvalidDateRange(&'static str),
    DateRangeTooLarge,
    InvalidLimit,
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
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

#[derive(Debug)]
pub(crate) struct ApiError;

impl ApiError {
    pub(crate) fn database(error: sqlx::Error) -> Self {
        tracing::error!(%error, "analytics database query failed");
        Self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
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

#[derive(Debug)]
pub(crate) struct HealthError;

impl HealthError {
    pub(crate) fn database(error: sqlx::Error) -> Self {
        tracing::error!(%error, "analytics health database check failed");
        Self
    }
}

impl IntoResponse for HealthError {
    fn into_response(self) -> Response {
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
pub(crate) enum HandlerError {
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
    fn into_response(self) -> Response {
        match self {
            Self::Request(error) => error.into_response(),
            Self::Api(error) => error.into_response(),
        }
    }
}
