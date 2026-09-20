import type {
  AnalyticsApiErrorResponse,
  OverviewResponse,
  PageItem,
  PagesResponse,
  RangeOverviewResponse,
  TimelineItem,
  TimelineResponse,
} from "./types";

export const DEFAULT_PAGES_LIMIT = 20;

export type ResponseValidator<T> = (value: unknown) => value is T;

export function overviewPath(siteId: string): string {
  return `/v1/sites/${encodeURIComponent(siteId)}/overview`;
}

export function rangeOverviewPath(siteId: string, from: string, to: string): string {
  return `/v1/sites/${encodeURIComponent(siteId)}/reports/${from}/${to}/overview`;
}

export function timelinePath(siteId: string, from: string, to: string): string {
  return `/v1/sites/${encodeURIComponent(siteId)}/reports/${from}/${to}/timeline`;
}

export function pagesPath(siteId: string, from: string, to: string, limit: number): string {
  return `/v1/sites/${encodeURIComponent(siteId)}/reports/${from}/${to}/pages?limit=${encodeURIComponent(String(limit))}`;
}

export function isOverviewResponse(value: unknown): value is OverviewResponse {
  return isRecord(value) && isString(value.site_id) && isNonNegativeInteger(value.page_views);
}

export function isRangeOverviewResponse(value: unknown): value is RangeOverviewResponse {
  return (
    isRecord(value) &&
    isString(value.site_id) &&
    isNonNegativeInteger(value.page_views) &&
    isString(value.from) &&
    isValidDate(value.from) &&
    isString(value.to) &&
    isValidDate(value.to)
  );
}

export function isTimelineResponse(value: unknown): value is TimelineResponse {
  return (
    isRecord(value) &&
    isString(value.site_id) &&
    isString(value.from) &&
    isValidDate(value.from) &&
    isString(value.to) &&
    isValidDate(value.to) &&
    Array.isArray(value.items) &&
    value.items.every(isTimelineItem)
  );
}

export function isPagesResponse(value: unknown): value is PagesResponse {
  return (
    isRecord(value) &&
    isString(value.site_id) &&
    isString(value.from) &&
    isValidDate(value.from) &&
    isString(value.to) &&
    isValidDate(value.to) &&
    Array.isArray(value.items) &&
    value.items.every(isPageItem)
  );
}

export function isAnalyticsApiErrorResponse(value: unknown): value is AnalyticsApiErrorResponse {
  return (
    isRecord(value) &&
    isRecord(value.error) &&
    isString(value.error.code) &&
    isString(value.error.message)
  );
}

function isTimelineItem(value: unknown): value is TimelineItem {
  return (
    isRecord(value) &&
    isString(value.day) &&
    isValidDate(value.day) &&
    isNonNegativeInteger(value.page_views)
  );
}

function isPageItem(value: unknown): value is PageItem {
  return (
    isRecord(value) &&
    isString(value.path) &&
    value.path.startsWith("/") &&
    isNonNegativeInteger(value.page_views)
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

function isNonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isValidDate(value: string): boolean {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    return false;
  }

  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(Date.UTC(year, month - 1, day));

  return (
    date.getUTCFullYear() === year && date.getUTCMonth() === month - 1 && date.getUTCDate() === day
  );
}
