"use client";

import { ReactRouterNavigationBridge } from "@web-analytics/observer-react-router";

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
    <ReactRouterNavigationBridge key={bindingVersion} observer={observer} now={now} />
  ) : null;
}
