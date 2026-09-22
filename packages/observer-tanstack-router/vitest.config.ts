import { defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "@web-analytics/observer-core": new URL("../observer-core/src/index.ts", import.meta.url)
        .pathname,
    },
  },
  test: { environment: "jsdom" },
});
