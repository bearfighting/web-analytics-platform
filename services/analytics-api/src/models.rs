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

#[derive(Debug, Serialize)]
pub(crate) struct VisitorSessionItem {
    pub(crate) day: NaiveDate,
    pub(crate) page_views: i64,
    pub(crate) unique_visitors: i64,
    pub(crate) sessions: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct VisitorSessionReportResponse {
    pub(crate) site_id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) page_views: i64,
    pub(crate) unique_visitors: i64,
    pub(crate) sessions: i64,
    pub(crate) items: Vec<VisitorSessionItem>,
    pub(crate) data_as_of: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) aggregation_version: i32,
}

#[derive(Debug, Serialize)]
pub(crate) struct DimensionItem {
    pub(crate) value: String,
    pub(crate) page_views: i64,
    pub(crate) unique_visitors: i64,
    pub(crate) sessions: i64,
}

#[derive(Debug, Serialize)]
pub(crate) struct DimensionReportResponse {
    pub(crate) site_id: String,
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) dimension: String,
    pub(crate) items: Vec<DimensionItem>,
    pub(crate) data_as_of: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) aggregation_version: i32,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct VisitorSessionRow {
    pub(crate) day: NaiveDate,
    pub(crate) page_views: i64,
    pub(crate) unique_visitors: i64,
    pub(crate) sessions: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct DimensionRow {
    pub(crate) value: String,
    pub(crate) page_views: i64,
    pub(crate) unique_visitors: i64,
    pub(crate) sessions: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct WatermarkRow {
    pub(crate) source_name: String,
    pub(crate) processed_received_watermark: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ActiveGeneration {
    pub(crate) generation_id: String,
    pub(crate) aggregation_version: i32,
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
