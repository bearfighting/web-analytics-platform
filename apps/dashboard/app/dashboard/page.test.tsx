import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";

import DashboardPage from "./page";

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
  vi.unstubAllGlobals();
});

describe("DashboardPage server contract", () => {
  it.each([
    ["unknown site", { site_id: "site_unknown" }, "The selected site is not configured"],
    ["partial date range", { from: "2026-09-01" }, "Both from and to dates are required"],
  ])("does not query Analytics API for %s", async (_caseName, searchParams, message) => {
    vi.stubEnv("DASHBOARD_SITES", "site_playground,site_alpha");
    vi.stubEnv("DASHBOARD_DEFAULT_SITE", "site_playground");
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);

    const element = await DashboardPage({ searchParams: Promise.resolve(searchParams) });
    const markup = renderToStaticMarkup(element);

    expect(markup).toContain(message);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
