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

export interface AnalyticsApiErrorResponse {
  error: {
    code: string;
    message: string;
  };
}
