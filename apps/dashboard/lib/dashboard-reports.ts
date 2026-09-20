import { createAnalyticsApiClient } from "./analytics-api/client";
import { getAnalyticsApiUrl } from "./analytics-api/config";
import { AnalyticsApiClientError } from "./analytics-api/errors";

import type { AnalyticsApiClient } from "./analytics-api/client";
import type { PagesResponse, TimelineResponse } from "./analytics-api/types";
import type { DashboardApiDependencies } from "./dashboard-dependencies";
import type { DashboardOverviewContext } from "./dashboard-overview";

export type DashboardReportState<T> =
  { status: "success"; data: T } | { status: "error"; error: AnalyticsApiClientError };

export interface DashboardReportsState {
  timeline: DashboardReportState<TimelineResponse>;
  pages: DashboardReportState<PagesResponse>;
}

export async function loadDashboardReports(
  context: DashboardOverviewContext,
  dependencies: DashboardApiDependencies = {},
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

    return { timeline: { status: "error", error }, pages: { status: "error", error } };
  }

  const [timeline, pages] = await Promise.all([
    settle(() => client.timeline(context.siteId, context.dateRange.from, context.dateRange.to)),
    settle(() => client.pages(context.siteId, context.dateRange.from, context.dateRange.to)),
  ]);

  return { timeline, pages };
}

async function settle<T>(request: () => Promise<T>): Promise<DashboardReportState<T>> {
  try {
    return { status: "success", data: await request() };
  } catch (cause) {
    return { status: "error", error: toAnalyticsApiClientError(cause) };
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
