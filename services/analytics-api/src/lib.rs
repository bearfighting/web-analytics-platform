mod errors;
mod handlers;
mod models;
mod queries;
mod routes;
mod state;
mod validation;

pub use state::{AppState, state};

use sqlx::postgres::PgPoolOptions;

pub fn connect(database_url: &str) -> Result<AppState, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(database_url)?;
    Ok(state(pool))
}

pub fn router(state: AppState) -> axum::Router {
    routes::router(state)
}

pub fn parse_date_range_for_test(from: &str, to: &str) -> Result<(), &'static str> {
    validation::parse_range(from, to)
        .map(|_| ())
        .map_err(|error| match error {
            errors::RequestError::InvalidDateRange(_) => "invalid_date_range",
            errors::RequestError::DateRangeTooLarge => "date_range_too_large",
            errors::RequestError::InvalidLimit => "invalid_limit",
        })
}

pub fn parse_limit_for_test(value: Option<&str>) -> Result<i64, &'static str> {
    validation::parse_limit(value).map_err(|_| "invalid_limit")
}
