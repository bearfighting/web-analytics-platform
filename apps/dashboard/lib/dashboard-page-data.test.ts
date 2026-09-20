import { describe, expect, it, vi } from "vitest";

import { AnalyticsApiClientError } from "./analytics-api/errors";
import { loadDashboardPageData } from "./dashboard-page-data";

import type { AnalyticsApiClient } from "./analytics-api/client";

const context = {
  siteId: "site_playground",
  dateRange: { from: "2026-09-18", to: "2026-09-18" },
};

function createClient(): AnalyticsApiClient {
  return {
    overview: vi.fn().mockResolvedValue({ site_id: context.siteId, page_views: 3 }),
    rangeOverview: vi.fn().mockResolvedValue({
      site_id: context.siteId,
      from: context.dateRange.from,
      to: context.dateRange.to,
      page_views: 3,
    }),
    timeline: vi.fn().mockResolvedValue({
      site_id: context.siteId,
      from: context.dateRange.from,
      to: context.dateRange.to,
      items: [],
    }),
    pages: vi.fn().mockResolvedValue({
      site_id: context.siteId,
      from: context.dateRange.from,
      to: context.dateRange.to,
      items: [],
    }),
  };
}

describe("loadDashboardPageData", () => {
  it("creates one client and reuses it for all dashboard queries", async () => {
    const client = createClient();
    const createClientMock = vi.fn(() => client);

    const result = await loadDashboardPageData(context, {
      getApiUrl: () => "http://analytics-api:4002",
      createClient: createClientMock,
    });

    expect(createClientMock).toHaveBeenCalledTimes(1);
    expect(client.overview).toHaveBeenCalledTimes(1);
    expect(client.rangeOverview).toHaveBeenCalledTimes(1);
    expect(client.timeline).toHaveBeenCalledTimes(1);
    expect(client.pages).toHaveBeenCalledTimes(1);
    expect(result.overview.status).toBe("success");
    expect(result.reports.timeline.status).toBe("success");
    expect(result.reports.pages.status).toBe("success");
  });

  it("returns contextual errors for all sections when configuration fails", async () => {
    const result = await loadDashboardPageData(context, {
      getApiUrl: () => {
        throw new AnalyticsApiClientError("Missing Analytics API URL", { kind: "config" });
      },
    });

    expect(result.overview).toMatchObject({ status: "error", context, error: { kind: "config" } });
    expect(result.reports.timeline).toMatchObject({ status: "error", error: { kind: "config" } });
    expect(result.reports.pages).toMatchObject({ status: "error", error: { kind: "config" } });
  });
});
