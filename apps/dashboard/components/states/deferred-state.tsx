import { dashboardStateCopy } from "./state-copy";

import type { DashboardStateContext } from "../../lib/dashboard-state";

interface DeferredStateProps {
  context: DashboardStateContext;
}

export function DeferredState({ context }: DeferredStateProps) {
  return <p data-site-id={context.siteId}>{dashboardStateCopy.deferred}</p>;
}
