import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  transpilePackages: [
    "@web-analytics/analytics-browser",
    "@web-analytics/analytics-core",
    "@web-analytics/observer-core",
    "@web-analytics/observer-next",
    "@web-analytics/protocol-ts",
  ],
};

export default nextConfig;
