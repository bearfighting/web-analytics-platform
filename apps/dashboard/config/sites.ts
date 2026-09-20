export interface DashboardSiteConfig {
  sites: string[];
  defaultSite: string;
}

export type DashboardSiteConfigError =
  "missing_sites" | "missing_default_site" | "default_site_not_allowed";

export interface DashboardSiteConfigResult {
  config?: DashboardSiteConfig;
  error?: DashboardSiteConfigError;
}

export interface DashboardEnvironment {
  DASHBOARD_SITES?: string;
  DASHBOARD_DEFAULT_SITE?: string;
}

function parseSites(value: string | undefined): string[] {
  return [
    ...new Set(
      (value ?? "")
        .split(",")
        .map((site) => site.trim())
        .filter(Boolean),
    ),
  ];
}

export function parseDashboardSiteConfig(
  environment: DashboardEnvironment,
): DashboardSiteConfigResult {
  const sites = parseSites(environment.DASHBOARD_SITES);
  const defaultSite = environment.DASHBOARD_DEFAULT_SITE?.trim() ?? "";

  if (sites.length === 0) {
    return { error: "missing_sites" };
  }

  if (!defaultSite) {
    return { error: "missing_default_site" };
  }

  if (!sites.includes(defaultSite)) {
    return { error: "default_site_not_allowed" };
  }

  return { config: { sites, defaultSite } };
}

export function getDashboardSiteConfig(): DashboardSiteConfigResult {
  return parseDashboardSiteConfig({
    DASHBOARD_SITES: process.env.DASHBOARD_SITES,
    DASHBOARD_DEFAULT_SITE: process.env.DASHBOARD_DEFAULT_SITE,
  });
}
