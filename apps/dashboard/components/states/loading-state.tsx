import { dashboardStateCopy } from "./state-copy";

import type { DashboardStateContext } from "../../lib/dashboard-state";

interface LoadingStateProps {
  context: DashboardStateContext;
}

export function LoadingState({ context }: LoadingStateProps) {
  return (
    <p role="status" data-site-id={context.siteId}>
      {dashboardStateCopy.loading}
    </p>
  );
}
