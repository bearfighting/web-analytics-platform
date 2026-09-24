import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { AnalyticsApiClientError } from "../lib/analytics-api/errors";

import { GeoCountryTable } from "./geo-country-table";

const context = { siteId: "site_example", dateRange: { from: "2026-09-01", to: "2026-09-02" } };
const response = {
  site_id: "site_example",
  from: "2026-09-01",
  to: "2026-09-02",
  coverage_from: "2026-09-01",
  items: [
    { country_code: "CA", page_views: 4 },
    { country_code: "unknown", page_views: 1 },
  ],
};

describe("GeoCountryTable", () => {
  it("shows country codes and unknown counts", () => {
    const html = renderToStaticMarkup(
      <GeoCountryTable context={context} state={{ status: "success", data: response }} />,
    );
    expect(html).toContain("CA");
    expect(html).toContain("Unknown");
    expect(html).toContain("Page Views by country");
  });

  it("shows empty and error states", () => {
    const empty = renderToStaticMarkup(
      <GeoCountryTable
        context={context}
        state={{ status: "success", data: { ...response, items: [] } }}
      />,
    );
    expect(empty).toContain("No page view data");
    const error = renderToStaticMarkup(
      <GeoCountryTable
        context={context}
        state={{
          status: "error",
          error: new AnalyticsApiClientError("Report failed", { kind: "network" }),
        }}
      />,
    );
    expect(error).toContain("Report failed");
  });
});
