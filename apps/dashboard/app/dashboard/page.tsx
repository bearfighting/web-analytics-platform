import { DashboardSections } from "../../components/dashboard-sections";
import { DashboardShell } from "../../components/dashboard-shell";
import { ErrorState } from "../../components/states/error-state";
import { getDashboardSiteConfig } from "../../config/sites";
import {
  defaultDashboardDateRange,
  parseDashboardQuery,
  type DashboardDateRange,
  type DashboardSearchParams,
} from "../../lib/query-params";

interface DashboardPageProps {
  searchParams: Promise<DashboardSearchParams>;
}

function firstValue(value: string | string[] | undefined): string | undefined {
  return Array.isArray(value) ? value[0] : value;
}

export default async function DashboardPage({ searchParams }: DashboardPageProps) {
  const siteConfig = getDashboardSiteConfig();

  if (siteConfig.error || !siteConfig.config) {
    return (
      <main className="dashboard-shell">
        <header className="dashboard-header">
          <p className="eyebrow">Web Analytics</p>
          <h1>Dashboard configuration</h1>
        </header>
        <section className="card">
          <ErrorState message={`Dashboard site configuration is invalid: ${siteConfig.error}.`} />
        </section>
      </main>
    );
  }

  const resolvedSearchParams = await searchParams;
  const query = parseDashboardQuery(
    resolvedSearchParams,
    siteConfig.config.defaultSite,
    siteConfig.config.sites,
  );

  if (query.error) {
    const requestedSite = firstValue(resolvedSearchParams.site_id) || siteConfig.config.defaultSite;
    const defaultDateRange = defaultDashboardDateRange(new Date());
    const displayedSite = siteConfig.config.sites.includes(requestedSite)
      ? requestedSite
      : siteConfig.config.defaultSite;
    const displayedDateRange: DashboardDateRange = {
      from: firstValue(resolvedSearchParams.from) || defaultDateRange.from,
      to: firstValue(resolvedSearchParams.to) || defaultDateRange.to,
    };

    return (
      <DashboardShell
        dateRange={displayedDateRange}
        siteId={displayedSite}
        sites={siteConfig.config.sites}
      >
        <section className="card">
          <ErrorState
            context={{ siteId: requestedSite, dateRange: displayedDateRange }}
            message={query.error.message}
          />
          <p className="context">
            Requested site: {requestedSite} · Requested range: {displayedDateRange.from} to{" "}
            {displayedDateRange.to} UTC
          </p>
        </section>
      </DashboardShell>
    );
  }

  return (
    <DashboardShell
      dateRange={query.params.dateRange}
      siteId={query.params.siteId}
      sites={siteConfig.config.sites}
    >
      <DashboardSections
        from={query.params.dateRange.from}
        siteId={query.params.siteId}
        to={query.params.dateRange.to}
      />
    </DashboardShell>
  );
}
