import React from "react";

import { loadDashboardOverview } from "../lib/dashboard-overview";
import { loadDashboardLegacyReports } from "../lib/dashboard-reports";

import { EventReportTable } from "./event-report-table";
import { OverviewCard } from "./overview-card";
import { ErrorState } from "./states/error-state";
import { TimelineTable } from "./timeline-table";
import { TopPagesTable } from "./top-pages-table";

import type { AnalyticsApiClient } from "../lib/analytics-api/client";
import type { DashboardOverviewContext } from "../lib/dashboard-overview";

interface LegacyDashboardSectionsProps {
  context: DashboardOverviewContext;
  client: AnalyticsApiClient;
}

export async function LegacyDashboardSections({ context, client }: LegacyDashboardSectionsProps) {
  const [overview, reports] = await Promise.all([
    loadDashboardOverview(context, { client }),
    loadDashboardLegacyReports(context, { client }),
  ]);

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
      <EventReportTable context={context} state={reports.events} />
    </>
  );
}
