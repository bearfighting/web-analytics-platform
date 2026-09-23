import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { RouterAnalyticsBridge as NextRouterAnalyticsBridge } from "@web-analytics/router-adapters/next";
import { RouterAnalyticsBridge as ReactRouterAnalyticsBridge } from "@web-analytics/router-adapters/react-router";
import { RouterAnalyticsBridge as TanStackRouterAnalyticsBridge } from "@web-analytics/router-adapters/tanstack-router";
import { describe, expect, it } from "vitest";

describe("Router adapter package exports", () => {
  it("exports RouterAnalyticsBridge from every subpath", () => {
    expect(NextRouterAnalyticsBridge).toBeTypeOf("function");
    expect(ReactRouterAnalyticsBridge).toBeTypeOf("function");
    expect(TanStackRouterAnalyticsBridge).toBeTypeOf("function");
  });

  it("exposes only the three Router subpaths", () => {
    const packageJson = JSON.parse(
      readFileSync(resolve(import.meta.dirname, "../package.json"), "utf8"),
    );

    expect(Object.keys(packageJson.exports).sort()).toEqual([
      "./next",
      "./react-router",
      "./tanstack-router",
    ]);
    expect(packageJson.exports["."]).toBeUndefined();
  });
});
