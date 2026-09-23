import { createAnalyticsApiClient } from "./analytics-api/client";
import { getAnalyticsApiUrl } from "./analytics-api/config";
import { AnalyticsApiClientError } from "./analytics-api/errors";

import type { AnalyticsApiClient } from "./analytics-api/client";
import type {
  AnalyticsDimension,
  DimensionResponse,
  EventsResponse,
  PagesResponse,
  TimelineResponse,
  VisitorSessionResponse,
} from "./analytics-api/types";
import type { DashboardApiDependencies } from "./dashboard-dependencies";
import type { DashboardOverviewContext } from "./dashboard-overview";

export type DashboardReportState<T> =
  | { status: "success"; data: T }
  | { status: "error"; error: AnalyticsApiClientError }
  | { status: "disabled"; error: AnalyticsApiClientError };

export interface DashboardReportsState {
  events: DashboardReportState<EventsResponse>;
  timeline: DashboardReportState<TimelineResponse>;
  pages: DashboardReportState<PagesResponse>;
  visitors: DashboardReportState<VisitorSessionResponse>;
  dimension: DashboardReportState<DimensionResponse>;
}

export interface DashboardLegacyReportsState {
  events: DashboardReportState<EventsResponse>;
  timeline: DashboardReportState<TimelineResponse>;
  pages: DashboardReportState<PagesResponse>;
}

export interface DashboardPhase6ReportsState {
  visitors: DashboardReportState<VisitorSessionResponse>;
  dimension: DashboardReportState<DimensionResponse>;
}

export async function loadDashboardLegacyReports(
  context: DashboardOverviewContext,
  dependencies: DashboardApiDependencies = {},
): Promise<DashboardLegacyReportsState> {
  let client: AnalyticsApiClient;

  try {
    client = resolveClient(dependencies);
  } catch (cause) {
    const error = toAnalyticsApiClientError(cause);

    return {
      timeline: { status: "error", error },
      pages: { status: "error", error },
      events: { status: "error", error },
    };
  }

  const [timeline, pages, events] = await Promise.all([
    settle(() => client.timeline(context.siteId, context.dateRange.from, context.dateRange.to)),
    settle(() => client.pages(context.siteId, context.dateRange.from, context.dateRange.to)),
    settle(() => client.events(context.siteId, context.dateRange.from, context.dateRange.to, 100)),
  ]);

  return { timeline, pages, events };
}

export async function loadDashboardPhase6Reports(
  context: DashboardOverviewContext,
  dependencies: DashboardApiDependencies = {},
  dimension: AnalyticsDimension = context.dimension ?? "browser",
): Promise<DashboardPhase6ReportsState> {
  let client: AnalyticsApiClient;

  try {
    client = resolveClient(dependencies);
  } catch (cause) {
    const error = toAnalyticsApiClientError(cause);

    return {
      visitors: { status: "error", error },
      dimension: { status: "error", error },
    };
  }

  const [visitors, dimensionReport] = await Promise.all([
    settlePhase6(() =>
      client.visitors(context.siteId, context.dateRange.from, context.dateRange.to),
    ),
    settlePhase6(() =>
      client.dimension(context.siteId, context.dateRange.from, context.dateRange.to, dimension),
    ),
  ]);

  return { visitors, dimension: dimensionReport };
}

export async function loadDashboardReports(
  context: DashboardOverviewContext,
  dependencies: DashboardApiDependencies = {},
  dimension: AnalyticsDimension = context.dimension ?? "browser",
): Promise<DashboardReportsState> {
  let client: AnalyticsApiClient;

  try {
    client =
      dependencies.client ??
      (dependencies.createClient ?? createAnalyticsApiClient)({
        baseUrl: (dependencies.getApiUrl ?? getAnalyticsApiUrl)(),
      });
  } catch (cause) {
    const error = toAnalyticsApiClientError(cause);

    return {
      timeline: { status: "error", error },
      pages: { status: "error", error },
      events: { status: "error", error },
      visitors: { status: "error", error },
      dimension: { status: "error", error },
    };
  }

  const [timeline, pages, events, visitors, dimensionReport] = await Promise.all([
    settle(() => client.timeline(context.siteId, context.dateRange.from, context.dateRange.to)),
    settle(() => client.pages(context.siteId, context.dateRange.from, context.dateRange.to)),
    settle(() => client.events(context.siteId, context.dateRange.from, context.dateRange.to, 100)),
    settlePhase6(() =>
      client.visitors(context.siteId, context.dateRange.from, context.dateRange.to),
    ),
    settlePhase6(() =>
      client.dimension(context.siteId, context.dateRange.from, context.dateRange.to, dimension),
    ),
  ]);

  return { timeline, pages, events, visitors, dimension: dimensionReport };
}

function resolveClient(dependencies: DashboardApiDependencies): AnalyticsApiClient {
  return (
    dependencies.client ??
    (dependencies.createClient ?? createAnalyticsApiClient)({
      baseUrl: (dependencies.getApiUrl ?? getAnalyticsApiUrl)(),
    })
  );
}

async function settle<T>(request: () => Promise<T>): Promise<DashboardReportState<T>> {
  try {
    return { status: "success", data: await request() };
  } catch (cause) {
    return { status: "error", error: toAnalyticsApiClientError(cause) };
  }
}

async function settlePhase6<T>(request: () => Promise<T>): Promise<DashboardReportState<T>> {
  try {
    return { status: "success", data: await request() };
  } catch (cause) {
    const error = toAnalyticsApiClientError(cause);

    return error.kind === "disabled" ? { status: "disabled", error } : { status: "error", error };
  }
}

function toAnalyticsApiClientError(cause: unknown): AnalyticsApiClientError {
  if (cause instanceof AnalyticsApiClientError) {
    return cause;
  }

  return new AnalyticsApiClientError("Dashboard report could not be loaded.", {
    kind: "network",
    cause,
  });
}
