/* global console, fetch, process, setTimeout */

import { execFileSync } from "node:child_process";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const composeFiles = ["-f", "compose.yaml", "-f", "compose.backend.yaml", "-f", "compose.e2e.yaml"];
const project = `web-analytics-dashboard-e2e-${process.pid}`;
const dashboardUrl = `http://127.0.0.1:${process.env.DASHBOARD_PORT ?? "13000"}`;
const errorDashboardPort = process.env.DASHBOARD_ERROR_E2E_PORT ?? "13001";
const errorContainer = `${project}-dashboard-error`;
const collectorUrl = `http://127.0.0.1:${process.env.E2E_COLLECTOR_PORT ?? "14001"}`;
const fixturesDirectory = path.join(root, "protocol", "phase-3", "fixtures");
const keys = {
  site_playground: "e2e-test-key",
  site_alpha: "e2e-test-key-alpha",
  site_beta: "e2e-test-key-beta",
};

const composeBaseArgs = [
  "compose",
  "-p",
  project,
  ...composeFiles,
  "--profile",
  "backend",
  "--profile",
  "storage",
  "--profile",
  "processing",
  "--profile",
  "dashboard",
];

const fixture = async (name) =>
  JSON.parse(await readFile(path.join(fixturesDirectory, `${name}.json`), "utf8"));

function runCompose(args, options = {}) {
  try {
    return execFileSync("docker", [...composeBaseArgs, ...args], {
      cwd: root,
      encoding: "utf8",
      stdio: options.capture ? ["ignore", "pipe", "inherit"] : "inherit",
    });
  } catch (error) {
    if (options.allowFailure) return "";
    throw new Error(`docker compose ${args.join(" ")} failed`, { cause: error });
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertDashboardIsolation() {
  const config = runCompose(["config"], { capture: true });
  const dashboardMatch = config.match(
    /\n\x20{2}dashboard:\n[\s\S]*?(?=\n\x20{2}[a-zA-Z0-9_-]+:|\nnetworks:|\nvolumes:)/,
  );
  assert(dashboardMatch, "Dashboard service is missing from Compose config");
  const dashboardConfig = dashboardMatch[0];
  assert(
    !/DATABASE_URL|postgres/i.test(dashboardConfig),
    "Dashboard Compose service must not connect to PostgreSQL",
  );
}

function expectedApi(data, name) {
  const response = data.expected?.api?.[name];
  assert(response?.status === 200, `Fixture is missing a successful ${name} API response`);
  return response.body;
}

function expectedSiteReport(data, siteId, from, to) {
  const daily = data.expected.page_view_daily.filter(
    (item) => item.site_id === siteId && item.day >= from && item.day <= to,
  );
  const routes = data.expected.page_view_routes
    .filter((item) => item.site_id === siteId && item.day >= from && item.day <= to)
    .reduce((items, item) => {
      const existing = items.find((candidate) => candidate.path === item.path);
      if (existing) existing.page_views += item.page_views;
      else items.push({ path: item.path, page_views: item.page_views });
      return items;
    }, [])
    .sort(
      (left, right) => right.page_views - left.page_views || left.path.localeCompare(right.path),
    );
  const pageViews =
    data.expected.page_view_totals.find((item) => item.site_id === siteId)?.page_views ?? 0;

  return {
    overview: { site_id: siteId, page_views: pageViews },
    rangeOverview: {
      site_id: siteId,
      from,
      to,
      page_views: daily.reduce((total, item) => total + item.page_views, 0),
    },
    timeline: {
      site_id: siteId,
      from,
      to,
      items: daily.map(({ day, page_views }) => ({ day, page_views })),
    },
    pages: { site_id: siteId, from, to, items: routes },
  };
}

async function assertDashboardRuntimeConfiguration() {
  const output = runCompose(
    ["exec", "-T", "dashboard", "sh", "-c", 'printf %s "$ANALYTICS_API_URL"'],
    { capture: true },
  ).trim();
  const expected = process.env.DASHBOARD_ANALYTICS_API_URL ?? "http://analytics-api:4002";
  assert(output === expected, `Dashboard is configured with ${output}, expected ${expected}`);
}

async function waitFor(label, url) {
  const deadline = Date.now() + 120_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The service may still be compiling or starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`${label} did not become ready at ${url}`);
}

async function resetDatabase() {
  runCompose([
    "exec",
    "-T",
    "postgres",
    "psql",
    "-U",
    "analytics",
    "-d",
    "analytics",
    "-v",
    "ON_ERROR_STOP=1",
    "-c",
    "TRUNCATE raw_events, page_view_daily, page_view_routes, page_view_totals RESTART IDENTITY",
  ]);
}

async function postFixtureEvents(input) {
  for (const siteId of new Set(input.events.map((event) => event.site_id))) {
    const events = input.events.filter((event) => event.site_id === siteId);
    const response = await fetch(`${collectorUrl}/v1/events`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        origin: "http://localhost:3000",
        "x-ingest-key": keys[siteId],
      },
      body: JSON.stringify({ schema_version: 1, events }),
    });
    const body = await response.json();
    assert(response.status === 202, `Collector rejected events: ${JSON.stringify(body)}`);
    assert(
      body.accepted === events.length,
      `Collector accepted ${body.accepted}, expected ${events.length}`,
    );
  }
}

function runProcessorOnce() {
  runCompose(
    [
      "run",
      "--rm",
      "--no-deps",
      "--build",
      "--entrypoint",
      "cargo",
      "processor",
      "run",
      "-p",
      "processor",
      "--",
      "--once",
    ],
    { capture: true },
  );
}

async function prepareFixture(data) {
  await resetDatabase();
  await postFixtureEvents(data.input);
  runProcessorOnce();
}

function rangeUrl(siteId, from, to) {
  return `${dashboardUrl}/dashboard?site_id=${siteId}&from=${from}&to=${to}`;
}

async function expectMetric(page, label, value) {
  const card = page.locator("article.card").filter({ hasText: label });
  await card
    .locator(".metric")
    .filter({ hasText: String(value) })
    .waitFor();
}

async function expectReportRows(page, sectionName, rows) {
  const section = page.locator("section.card").filter({ hasText: sectionName });
  const actual = await section
    .locator("tbody tr")
    .evaluateAll((rows) =>
      rows.map((row) =>
        Array.from(row.querySelectorAll("td"), (cell) => cell.textContent?.trim() ?? "").join(" "),
      ),
    );
  assert(
    JSON.stringify(actual) === JSON.stringify(rows),
    `${sectionName} rows mismatch: ${JSON.stringify(actual)}`,
  );
}

async function assertSinglePageView(page) {
  const data = await fixture("single-page-view");
  const overview = expectedApi(data, "overview");
  const rangeOverview = expectedApi(data, "range_overview");
  const timeline = expectedApi(data, "timeline");
  const pages = expectedApi(data, "pages");
  await prepareFixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-18", "2026-09-18"));
  await expectMetric(page, "Site total Page Views", overview.page_views);
  await expectMetric(page, "Selected range Page Views", rangeOverview.page_views);
  await expectReportRows(
    page,
    "Timeline",
    timeline.items.map((item) => `${item.day} ${item.page_views}`),
  );
  await expectReportRows(
    page,
    "Top Pages",
    pages.items.map((item) => `${item.path} ${item.page_views}`),
  );
}

async function assertMultiPageNavigation(page) {
  const data = await fixture("multi-page-navigation");
  const rangeOverview = expectedApi(data, "range_overview");
  const pages = expectedApi(data, "pages");
  await prepareFixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-18", "2026-09-18"));
  await expectMetric(page, "Selected range Page Views", rangeOverview.page_views);
  await expectReportRows(
    page,
    "Top Pages",
    pages.items.map((item) => `${item.path} ${item.page_views}`),
  );
}

async function assertMultiSiteIsolation(page) {
  const data = await fixture("multi-site-isolation");
  const alpha = {
    overview: expectedApi(data, "overview"),
    rangeOverview: expectedApi(data, "range_overview"),
    pages: expectedApi(data, "pages"),
  };
  const beta = expectedSiteReport(data, "site_beta", "2026-09-18", "2026-09-18");
  await prepareFixture(data);
  await page.goto(rangeUrl("site_alpha", "2026-09-18", "2026-09-18"));
  await expectMetric(page, "Site total Page Views", alpha.overview.page_views);
  await page.locator('select[name="site_id"]').selectOption("site_beta");
  await page.locator('button[type="submit"]').click();
  await page.waitForURL(/site_id=site_beta/);
  await expectMetric(page, "Selected range Page Views", beta.rangeOverview.page_views);
  await expectReportRows(
    page,
    "Top Pages",
    beta.pages.items.map((item) => `${item.path} ${item.page_views}`),
  );
}

async function assertEmptyRange(page) {
  const data = await fixture("empty-date-range");
  const rangeOverview = expectedApi(data, "range_overview");
  await prepareFixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-01", "2026-09-01"));
  await expectMetric(page, "Selected range Page Views", rangeOverview.page_views);
  await page.getByText("No page view data is available for this selection.").nth(0).waitFor();
  assert(
    (await page.getByText("No page view data is available for this selection.").count()) === 2,
    "Expected empty states for Timeline and Top Pages",
  );
}

async function assertCustomDateRange(page) {
  const data = await fixture("single-page-view");
  const rangeOverview = expectedApi(data, "range_overview");
  await prepareFixture(data);
  await page.goto(`${dashboardUrl}/dashboard?site_id=site_playground`);
  await page.locator('input[name="from"]').fill("2026-09-18");
  await page.locator('input[name="to"]').fill("2026-09-18");
  await page.locator('button[type="submit"]').click();
  await page.waitForURL(/from=2026-09-18&to=2026-09-18/);
  await expectMetric(page, "Selected range Page Views", rangeOverview.page_views);
}

async function assertApiError(browser) {
  runCompose(
    [
      "run",
      "-d",
      "--no-deps",
      "--name",
      errorContainer,
      "-p",
      `${errorDashboardPort}:3000`,
      "-e",
      "ANALYTICS_API_URL=http://analytics-api:4999",
      "dashboard",
    ],
    { capture: true },
  );
  try {
    const errorUrl = `http://127.0.0.1:${errorDashboardPort}`;
    await waitFor("Dashboard error instance", `${errorUrl}/dashboard`);
    const page = await browser.newPage();
    await page.goto(`${errorUrl}/dashboard?site_id=site_playground&from=2026-09-18&to=2026-09-18`);
    await page.getByRole("alert").first().waitFor();
    assert(
      (await page.getByRole("alert").first().textContent()).includes("Analytics API"),
      "Missing API error state",
    );
    await page.getByText("Site: site_playground · 2026-09-18 to 2026-09-18 UTC").first().waitFor();
    await page.close();
  } finally {
    execFileSync("docker", ["rm", "-f", errorContainer], { cwd: root, stdio: "ignore" });
  }
}

let browser;
let browserContext;
let page;
try {
  assertDashboardIsolation();
  runCompose([
    "up",
    "-d",
    "--build",
    "--wait",
    "postgres",
    "collector",
    "analytics-api",
    "dashboard",
  ]);
  await waitFor("Dashboard", `${dashboardUrl}/dashboard`);
  await assertDashboardRuntimeConfiguration();
  browser = await chromium.launch({ headless: true });
  browserContext = await browser.newContext();
  await browserContext.tracing.start({ screenshots: true, snapshots: true });
  page = await browserContext.newPage();

  await assertSinglePageView(page);
  console.log("PASS single-page-view dashboard");
  await assertMultiPageNavigation(page);
  console.log("PASS multi-page-navigation dashboard");
  await assertMultiSiteIsolation(page);
  console.log("PASS multi-site-isolation dashboard");
  await assertEmptyRange(page);
  console.log("PASS empty-date-range dashboard");
  await assertCustomDateRange(page);
  console.log("PASS custom-date-range dashboard");
  await assertApiError(browser);
  console.log("PASS api-error dashboard");
  await page.close();
  console.log("Dashboard E2E workflow passed.");
} catch (error) {
  if (page || browserContext) {
    const artifactDirectory = path.join(root, "artifacts", "dashboard-e2e");
    await mkdir(artifactDirectory, { recursive: true });
    if (page)
      await page.screenshot({ path: path.join(artifactDirectory, "failure.png"), fullPage: true });
    if (browserContext)
      await browserContext.tracing.stop({ path: path.join(artifactDirectory, "trace.zip") });
    console.error(`Dashboard E2E artifacts saved in ${artifactDirectory}`);
  }
  console.error(error instanceof Error ? (error.stack ?? error.message) : String(error));
  runCompose(["logs", "--no-color", "dashboard", "collector", "analytics-api"], {
    allowFailure: true,
  });
  process.exitCode = 1;
} finally {
  if (browserContext) await browserContext.close();
  if (browser) await browser.close();
  runCompose(["down", "--volumes", "--remove-orphans"], { allowFailure: true });
}
