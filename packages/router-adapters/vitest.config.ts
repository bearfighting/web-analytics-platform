import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "@web-analytics/analytics-browser": new URL(
        "../analytics-browser/src/index.ts",
        import.meta.url,
      ).pathname,
      "@web-analytics/observer-core": new URL("../observer-core/src/index.ts", import.meta.url)
        .pathname,
      "@web-analytics/observer-next": new URL("../observer-next/src/index.ts", import.meta.url)
        .pathname,
      "@web-analytics/observer-react-router": new URL(
        "../observer-react-router/src/index.ts",
        import.meta.url,
      ).pathname,
      "@web-analytics/observer-tanstack-router": new URL(
        "../observer-tanstack-router/src/index.ts",
        import.meta.url,
      ).pathname,
    },
  },
  test: { environment: "jsdom" },
});
