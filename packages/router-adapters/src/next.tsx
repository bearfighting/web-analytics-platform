"use client";

import { NextNavigationBridge } from "@web-analytics/observer-next";

import { useRouterAnalyticsBridge } from "./bridge";

import type { RouterAnalyticsBridgeProps } from "./bridge";

export type { RouterAnalyticsBridgeProps } from "./bridge";

export function RouterAnalyticsBridge({
  analytics,
  onNavigation,
  now,
}: RouterAnalyticsBridgeProps) {
  const { bindingVersion, observer, ready } = useRouterAnalyticsBridge(analytics, onNavigation);

  return ready ? <NextNavigationBridge key={bindingVersion} observer={observer} now={now} /> : null;
}
