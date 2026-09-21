import React from "react";

import { ANALYTICS_DIMENSIONS } from "../lib/analytics-api/types";

import type { AnalyticsDimension } from "../lib/analytics-api/types";
import type { DashboardDateRange } from "../lib/query-params";
import type { ReactNode } from "react";

interface DashboardShellProps {
  dateRange: DashboardDateRange;
  siteId: string;
  sites: string[];
  dimension: AnalyticsDimension;
  children: ReactNode;
}

export function DashboardShell({
  dateRange,
  siteId,
  sites,
  dimension,
  children,
}: DashboardShellProps) {
  return (
    <main className="dashboard-shell">
      <header className="dashboard-header">
        <div>
          <p className="eyebrow">Web Analytics</p>
          <h1>Dashboard</h1>
          <p>Page view activity for the selected site and UTC date range.</p>
        </div>
        <form action="/dashboard" method="get" className="filters">
          <label>
            Site
            <select name="site_id" defaultValue={siteId}>
              {sites.map((site) => (
                <option key={site} value={site}>
                  {site}
                </option>
              ))}
            </select>
          </label>
          <label>
            From
            <input name="from" type="date" defaultValue={dateRange.from} />
          </label>
          <label>
            To
            <input name="to" type="date" defaultValue={dateRange.to} />
          </label>
          <label>
            Dimension
            <select name="dimension" defaultValue={dimension}>
              {ANALYTICS_DIMENSIONS.map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
          <button type="submit">Apply</button>
        </form>
      </header>
      {children}
    </main>
  );
}
