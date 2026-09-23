export interface OverviewResponse {
  site_id: string;
  page_views: number;
}

export interface RangeOverviewResponse extends OverviewResponse {
  from: string;
  to: string;
}

export interface TimelineItem {
  day: string;
  page_views: number;
}

export interface TimelineResponse {
  site_id: string;
  from: string;
  to: string;
  items: TimelineItem[];
}

export interface PageItem {
  path: string;
  page_views: number;
}

export interface PagesResponse {
  site_id: string;
  from: string;
  to: string;
  items: PageItem[];
}

export const ANALYTICS_DIMENSIONS = [
  "language",
  "timezone",
  "utm_source",
  "utm_medium",
  "utm_campaign",
  "utm_term",
  "utm_content",
  "referrer_host",
  "device",
  "browser",
  "os",
] as const;

export type AnalyticsDimension = (typeof ANALYTICS_DIMENSIONS)[number];
export type FreshnessStatus = "current" | "stale" | "rebuilding" | "failed";

export interface VisitorSessionItem {
  day: string;
  page_views: number;
  unique_visitors: number;
  sessions: number;
}

export interface VisitorSessionResponse {
  site_id: string;
  from: string;
  to: string;
  page_views: number;
  unique_visitors: number;
  sessions: number;
  items: VisitorSessionItem[];
  data_as_of: string | null;
  freshness_status: FreshnessStatus;
  aggregation_version: number;
}

export interface DimensionItem {
  value: string;
  page_views: number;
  unique_visitors: number;
  sessions: number;
}

export interface DimensionResponse {
  site_id: string;
  from: string;
  to: string;
  dimension: AnalyticsDimension;
  items: DimensionItem[];
  data_as_of: string | null;
  freshness_status: FreshnessStatus;
  aggregation_version: number;
}

export interface AnalyticsApiErrorResponse {
  error: {
    code: string;
    message: string;
  };
}

export interface EventDailyItem {
  day: string;
  event_name: string;
  event_count: number;
}

export interface EventsResponse {
  site_id: string;
  from: string;
  to: string;
  total: number;
  items: EventDailyItem[];
  data_as_of: string | null;
  freshness_status: FreshnessStatus;
  aggregation_version: number;
}
