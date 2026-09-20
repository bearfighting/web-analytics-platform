import { loadDashboardPageData } from "../lib/dashboard-page-data";

import { OverviewCard } from "./overview-card";
import { ErrorState } from "./states/error-state";
import { TimelineTable } from "./timeline-table";
import { TopPagesTable } from "./top-pages-table";

interface DashboardSectionsProps {
  siteId: string;
  from: string;
  to: string;
}

export async function DashboardSections({ siteId, from, to }: DashboardSectionsProps) {
  const context = { siteId, dateRange: { from, to } };
  const { overview, reports } = await loadDashboardPageData(context);

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
      <TimelineTable context={context} state={reports.timeline} />
      <TopPagesTable context={context} state={reports.pages} />
    </>
  );
}
