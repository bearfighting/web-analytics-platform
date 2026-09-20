import { describe, expect, it } from "vitest";

import { parseDashboardSiteConfig } from "./sites";

describe("dashboard site configuration", () => {
  it("trims and deduplicates configured sites", () => {
    expect(
      parseDashboardSiteConfig({
        DASHBOARD_SITES: " site_playground,site_alpha,site_playground,, ",
        DASHBOARD_DEFAULT_SITE: " site_alpha ",
      }),
    ).toEqual({
      config: { sites: ["site_playground", "site_alpha"], defaultSite: "site_alpha" },
    });
  });

  it.each([
    ["missing_sites", {}],
    ["missing_default_site", { DASHBOARD_SITES: "site_playground" }],
    [
      "default_site_not_allowed",
      { DASHBOARD_SITES: "site_playground", DASHBOARD_DEFAULT_SITE: "site_alpha" },
    ],
  ])("rejects %s", (error, environment) => {
    expect(parseDashboardSiteConfig(environment)).toEqual({ error });
  });
});
