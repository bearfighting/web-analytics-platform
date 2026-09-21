import React from "react";

import { loadDashboardPhase6Reports } from "../lib/dashboard-reports";

import { DimensionReportTable } from "./dimension-report-table";
import { FreshnessBanner } from "./freshness-banner";
import { OverviewCard } from "./overview-card";
import { VisitorSessionTrendTable } from "./visitor-session-trend-table";

import type { AnalyticsApiClient } from "../lib/analytics-api/client";
import type { DashboardOverviewContext } from "../lib/dashboard-overview";

interface Phase6DashboardSectionsProps {
  context: DashboardOverviewContext;
  client: AnalyticsApiClient;
}

export async function Phase6DashboardSections({ context, client }: Phase6DashboardSectionsProps) {
  const reports = await loadDashboardPhase6Reports(context, { client });

  return (
    <>
      {reports.visitors.status === "success" ? (
        <section className="overview-grid" aria-label="Phase 6 Overview">
          <OverviewCard
            context={context}
            label="Unique Visitors"
            pageViews={reports.visitors.data.unique_visitors}
          />
          <OverviewCard
            context={context}
            label="Sessions"
            pageViews={reports.visitors.data.sessions}
          />
        </section>
      ) : null}
      <FreshnessBanner
        context={context}
        visitorState={reports.visitors}
        dimensionState={reports.dimension}
      />
      <VisitorSessionTrendTable context={context} state={reports.visitors} />
      <DimensionReportTable context={context} state={reports.dimension} />
    </>
  );
}
