import { readFileSync } from "node:fs";

import { describe, expect, it, vi } from "vitest";

import { AnalyticsApiClientError } from "./analytics-api/errors";
import { loadDashboardReports } from "./dashboard-reports";

import type { AnalyticsApiClient } from "./analytics-api/client";

const context = {
  siteId: "site_playground",
  dateRange: { from: "2026-09-18", to: "2026-09-18" },
};

const fixture = readFixture();
const emptyFixture = readFixture("empty-date-range.json");

function createClient(overrides: Partial<AnalyticsApiClient> = {}): AnalyticsApiClient {
  return {
    overview: vi.fn(),
    rangeOverview: vi.fn(),
    timeline: vi.fn().mockResolvedValue(fixture.timeline),
    pages: vi.fn().mockResolvedValue(fixture.pages),
    ...overrides,
  };
}

function readFixture(filename = "multi-page-navigation.json") {
  const value = JSON.parse(
    readFileSync(
      new URL(`../../../protocol/phase-3/fixtures/${filename}`, import.meta.url),
      "utf8",
    ),
  ) as {
    expected: {
      api: {
        timeline: { body: unknown };
        pages: { body: unknown };
      };
    };
  };

  return { timeline: value.expected.api.timeline.body, pages: value.expected.api.pages.body };
}

describe("loadDashboardReports", () => {
  it("queries timeline and pages in parallel with the current context", async () => {
    const client = createClient();
    const resultPromise = loadDashboardReports(context, { client });

    expect(client.timeline).toHaveBeenCalledWith("site_playground", "2026-09-18", "2026-09-18");
    expect(client.pages).toHaveBeenCalledWith("site_playground", "2026-09-18", "2026-09-18");

    const result = await resultPromise;

    expect(result).toEqual({
      timeline: { status: "success", data: fixture.timeline },
      pages: { status: "success", data: fixture.pages },
    });
  });

  it("keeps timeline and pages errors independent", async () => {
    const timelineError = new AnalyticsApiClientError("Timeline failed", {
      kind: "http",
      status: 503,
    });
    const pagesError = new AnalyticsApiClientError("Pages failed", {
      kind: "http",
      status: 502,
    });
    const timelineClient = createClient({ timeline: vi.fn().mockRejectedValue(timelineError) });
    const pagesClient = createClient({ pages: vi.fn().mockRejectedValue(pagesError) });

    await expect(loadDashboardReports(context, { client: timelineClient })).resolves.toMatchObject({
      timeline: { status: "error", error: timelineError },
      pages: { status: "success" },
    });
    await expect(loadDashboardReports(context, { client: pagesClient })).resolves.toMatchObject({
      timeline: { status: "success" },
      pages: { status: "error", error: pagesError },
    });
  });

  it("preserves empty canonical report responses", async () => {
    const client = createClient({
      timeline: vi.fn().mockResolvedValue(emptyFixture.timeline),
      pages: vi.fn().mockResolvedValue(emptyFixture.pages),
    });

    const result = await loadDashboardReports(context, { client });

    expect(result).toEqual({
      timeline: { status: "success", data: emptyFixture.timeline },
      pages: { status: "success", data: emptyFixture.pages },
    });
  });

  it("returns independent config errors without making requests", async () => {
    const clientFactory = vi.fn();

    const result = await loadDashboardReports(context, {
      getApiUrl: () => {
        throw new AnalyticsApiClientError("Missing Analytics API URL", { kind: "config" });
      },
      createClient: clientFactory,
    });

    expect(result.timeline.status).toBe("error");
    expect(result.pages.status).toBe("error");
    expect(clientFactory).not.toHaveBeenCalled();
  });
});
