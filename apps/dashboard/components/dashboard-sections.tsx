import React, { Suspense } from "react";

import { createAnalyticsApiClient } from "../lib/analytics-api/client";
import { getAnalyticsApiUrl } from "../lib/analytics-api/config";
import { AnalyticsApiClientError } from "../lib/analytics-api/errors";

import { LegacyDashboardSections } from "./legacy-dashboard-sections";
import { Phase6DashboardSections } from "./phase6-dashboard-sections";
import { Phase6LoadingState } from "./phase6-loading-state";
import { ErrorState } from "./states/error-state";
import { LoadingState } from "./states/loading-state";

import type { AnalyticsApiClient } from "../lib/analytics-api/client";
import type { AnalyticsDimension } from "../lib/analytics-api/types";

interface DashboardSectionsProps {
  siteId: string;
  from: string;
  to: string;
  dimension: AnalyticsDimension;
}

export function DashboardSections({ siteId, from, to, dimension }: DashboardSectionsProps) {
  const context = { siteId, dateRange: { from, to }, dimension };
  let client: AnalyticsApiClient;

  try {
    client = createAnalyticsApiClient({ baseUrl: getAnalyticsApiUrl() });
  } catch (cause) {
    const error =
      cause instanceof AnalyticsApiClientError
        ? cause
        : new AnalyticsApiClientError("Dashboard data could not be loaded.", {
            kind: "config",
            cause,
          });

    return (
      <>
        <section className="card" aria-label="Overview">
          <h2>Overview</h2>
          <ErrorState context={context} message={error.message} />
        </section>
        <section className="card" aria-label="Phase 6 analytics">
          <ErrorState context={context} message={error.message} />
        </section>
      </>
    );
  }

  return (
    <>
      <Suspense
        fallback={
          <section className="card" aria-label="Overview">
            <LoadingState context={context} />
          </section>
        }
      >
        <LegacyDashboardSections context={context} client={client} />
      </Suspense>
      <Suspense
        fallback={
          <>
            <Phase6LoadingState heading="Visitors and Sessions" />
            <Phase6LoadingState heading="Dimension Report" />
          </>
        }
      >
        <Phase6DashboardSections context={context} client={client} />
      </Suspense>
    </>
  );
}
