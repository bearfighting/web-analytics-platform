import { dashboardStateCopy } from "./state-copy";

import type { DashboardStateContext } from "../../lib/dashboard-state";

interface ErrorStateProps {
  context?: DashboardStateContext;
  message?: string;
}

export function ErrorState({ context, message }: ErrorStateProps) {
  return (
    <p role="alert" data-site-id={context?.siteId}>
      {message ?? dashboardStateCopy.error}
    </p>
  );
}
