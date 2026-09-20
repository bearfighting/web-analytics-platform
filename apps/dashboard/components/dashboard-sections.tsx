import { EmptyState } from "./states/empty-state";
import { SuccessState } from "./states/success-state";

import type { DashboardStateContext } from "../lib/dashboard-state";

interface DashboardSectionsProps {
  siteId: string;
  from: string;
  to: string;
}

export function DashboardSections({ siteId, from, to }: DashboardSectionsProps) {
  const context: DashboardStateContext = { siteId, dateRange: { from, to } };

  return (
    <>
      <section className="overview-grid" aria-label="Overview">
        <article className="card">
          <h2>Page Views</h2>
          <SuccessState context={context} />
          <p className="context">Site total · {siteId}</p>
        </article>
        <article className="card">
          <h2>Selected range</h2>
          <SuccessState context={context} />
          <p className="context">
            {from} to {to} UTC
          </p>
        </article>
      </section>
      <section className="card" aria-labelledby="timeline-heading">
        <h2 id="timeline-heading">Timeline</h2>
        <EmptyState context={context} />
      </section>
      <section className="card" aria-labelledby="top-pages-heading">
        <h2 id="top-pages-heading">Top Pages</h2>
        <EmptyState context={context} />
      </section>
    </>
  );
}
