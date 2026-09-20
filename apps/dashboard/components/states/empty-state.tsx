import React from "react";

import { dashboardStateCopy } from "./state-copy";

import type { DashboardStateContext } from "../../lib/dashboard-state";

interface EmptyStateProps {
  context: DashboardStateContext;
}

export function EmptyState({ context }: EmptyStateProps) {
  return <p data-site-id={context.siteId}>{dashboardStateCopy.empty}</p>;
}
