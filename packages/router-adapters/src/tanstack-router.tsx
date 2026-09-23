"use client";

import { TanStackRouterNavigationBridge } from "@web-analytics/observer-tanstack-router";

import { useRouterAnalyticsBridge } from "./bridge";

import type { RouterAnalyticsBridgeProps } from "./bridge";

export type { RouterAnalyticsBridgeProps } from "./bridge";

export function RouterAnalyticsBridge({
  analytics,
  onNavigation,
  now,
}: RouterAnalyticsBridgeProps) {
  const { bindingVersion, observer, ready } = useRouterAnalyticsBridge(analytics, onNavigation);

  return ready ? (
    <TanStackRouterNavigationBridge key={bindingVersion} observer={observer} now={now} />
  ) : null;
}
