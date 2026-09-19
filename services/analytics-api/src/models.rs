use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct HealthResponse {
    pub(crate) status: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct OverviewResponse {
    pub(crate) site_id: String,
    pub(crate) page_views: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct RangeOverviewResponse {
    pub(crate) site_id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) page_views: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct TimelineItem {
    pub(crate) day: NaiveDate,
    pub(crate) page_views: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct TimelineResponse {
    pub(crate) site_id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) items: Vec<TimelineItem>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PageItem {
    pub(crate) path: String,
    pub(crate) page_views: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct PagesResponse {
    pub(crate) site_id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) items: Vec<PageItem>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DateRange {
    pub(crate) from: NaiveDate,
    pub(crate) to: NaiveDate,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct TimelineRow {
    pub(crate) day: NaiveDate,
    pub(crate) page_views: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct PageRow {
    pub(crate) path: String,
    pub(crate) page_views: i64,
}
