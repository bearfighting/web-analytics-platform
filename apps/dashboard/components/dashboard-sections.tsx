import { loadDashboardOverview } from "../lib/dashboard-overview";

import { OverviewCard } from "./overview-card";
import { DeferredState } from "./states/deferred-state";
import { ErrorState } from "./states/error-state";

interface DashboardSectionsProps {
  siteId: string;
  from: string;
  to: string;
}

export async function DashboardSections({ siteId, from, to }: DashboardSectionsProps) {
  const context = { siteId, dateRange: { from, to } };
  const overview = await loadDashboardOverview(context);

  return (
    <>
      {overview.status === "error" ? (
        <section className="card" aria-label="Overview">
          <h2>Overview</h2>
          <ErrorState context={context} message={overview.error.message} />
        </section>
      ) : (
        <section className="overview-grid" aria-label="Overview">
          <OverviewCard
            context={context}
            label="Site total Page Views"
            pageViews={overview.data.overview.page_views}
          />
          <OverviewCard
            context={context}
            label="Selected range Page Views"
            pageViews={overview.data.rangeOverview.page_views}
          />
        </section>
      )}
      <section className="card" aria-labelledby="timeline-heading">
        <h2 id="timeline-heading">Timeline</h2>
        <DeferredState context={context} />
      </section>
      <section className="card" aria-labelledby="top-pages-heading">
        <h2 id="top-pages-heading">Top Pages</h2>
        <DeferredState context={context} />
      </section>
    </>
  );
}
