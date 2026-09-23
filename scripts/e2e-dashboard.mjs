/* global console, fetch, process, setTimeout */

import { execFileSync } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
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
const analyticsUrl = `http://127.0.0.1:${process.env.E2E_ANALYTICS_API_PORT ?? "14002"}`;
const fixturesDirectory = path.join(
  root,
  "protocol",
  "contracts",
  "analytics-api",
  "current",
  "fixtures",
);
const phase6FixturesDirectory = path.join(root, "tests", "fixtures", "dashboard");
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
  "--profile",
  "playground-next",
];

const fixture = async (name) =>
  JSON.parse(await readFile(path.join(fixturesDirectory, `${name}.json`), "utf8"));
const phase6Fixture = async (name) =>
  JSON.parse(await readFile(path.join(phase6FixturesDirectory, `${name}.json`), "utf8"));

function runCompose(args, options = {}) {
  try {
    return execFileSync("docker", [...composeBaseArgs, ...args], {
      cwd: root,
      encoding: "utf8",
      maxBuffer: 50 * 1024 * 1024,
      env: options.env ? { ...process.env, ...options.env } : process.env,
      stdio: options.capture ? ["ignore", "pipe", "pipe"] : "inherit",
    });
  } catch (error) {
    if (options.allowFailure) return `${error.stdout ?? ""}${error.stderr ?? ""}`;

    const stdout = error.stdout ?? "";
    const stderr = error.stderr ?? "";
    const status = error.status == null ? "unknown" : String(error.status);
    const output = [stdout, stderr].filter(Boolean).join("\n").trim();
    throw new Error(
      [
        `docker compose ${args.join(" ")} failed (exit code: ${status})`,
        output ? `Docker output:\n${output}` : "Docker produced no captured output.",
      ].join("\n"),
      { cause: error },
    );
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
  const deadline = Date.now() + 300_000;
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
    "TRUNCATE dimension_event_facts, dimension_daily, session_events, sessions, visitor_event_facts, visitor_daily, session_daily, normalized_event_context, web_vital_facts, analytics_rebuild_queue, analytics_watermarks, analytics_generations, analytics_feature_flags, raw_events, page_view_daily, page_view_routes, page_view_totals RESTART IDENTITY CASCADE",
  ]);
}

function enablePhase6(siteId) {
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
    `INSERT INTO analytics_feature_flags (site_id, analytics_enabled) VALUES ('${siteId}', TRUE)`,
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

async function postDimensionFixtureEvents(input) {
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
    assert(response.status === 202, `Collector rejected dimension events: ${JSON.stringify(body)}`);
    assert(
      body.accepted === events.length,
      `Collector accepted ${body.accepted} dimension events, expected ${events.length}`,
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

function runProcessorBackfill(from, to) {
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
      "--backfill",
      "--site-id",
      "site_playground",
      "--from",
      from,
      "--to",
      to,
    ],
    { capture: true },
  );
}

async function prepareFixture(data) {
  await resetDatabase();
  await postFixtureEvents(data.input);
  runProcessorOnce();
}

async function preparePhase6Fixture(data) {
  await resetDatabase();
  enablePhase6("site_playground");
  await postDimensionFixtureEvents(data.input);
  runProcessorOnce();
  runProcessorBackfill("2026-09-20", "2026-09-21");
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

async function assertBrowserWebVitalsCollection(browser) {
  await resetDatabase();
  const context = await browser.newContext();
  const page = await context.newPage();
  await page.addInitScript(() => {
    const nativeFetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      if (init?.keepalive) localStorage.setItem("__e2e_web_vitals_keepalive", "true");
      return nativeFetch(input, init);
    };
  });

  try {
    await page.goto("http://localhost:3000", { waitUntil: "domcontentloaded" });
    const navigationLog = page.getByTestId("events-json");
    await navigationLog.waitFor();
    await page.waitForFunction(() => {
      const value = document.querySelector("[data-testid=events-json]")?.textContent ?? "[]";
      return value.includes("initial");
    });
    await page.waitForTimeout(1200);
    await page.goto("http://localhost:3000/about", { waitUntil: "domcontentloaded" });
    assert(
      (await page.evaluate(() => localStorage.getItem("__e2e_web_vitals_keepalive"))) === "true",
      "The real Browser SDK did not issue a keepalive request when the document was hidden",
    );

    const deadline = Date.now() + 15_000;
    let payloads = [];
    while (Date.now() < deadline) {
      payloads = readBrowserEvents();
      if (payloads.some((event) => event.type === "web_vital")) break;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    const pageViews = new Map(
      payloads
        .filter((event) => event.type === "page_view")
        .map((event) => [event.event_id, event]),
    );
    const vitals = payloads.filter((event) => event.type === "web_vital");
    assert(vitals.length > 0, "The browser did not deliver any Web Vital events to the Collector");
    for (const event of vitals) {
      const pageView = pageViews.get(event.page_view_event_id);
      assert(pageView, `Web Vital ${event.event_id} is missing its Page View`);
      assert(
        event.path === pageView.path &&
          event.page_view_occurred_at === pageView.occurred_at &&
          event.occurred_at >= pageView.occurred_at,
        `Web Vital ${event.event_id} does not match its Page View fields`,
      );
      assert(
        ["LCP", "INP", "CLS", "FCP", "TTFB"].includes(event.metric),
        `Unexpected browser metric ${event.metric}`,
      );
      const max = event.metric === "CLS" ? 100 : 600_000;
      assert(
        Number.isFinite(event.value) && event.value >= 0 && event.value <= max,
        "Browser Web Vital value is outside protocol bounds",
      );
      assert(
        Number.isInteger(event.report_sequence) && event.report_sequence > 0,
        "Browser Web Vital report sequence is invalid",
      );
    }

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
    const factCount = Number(
      runCompose(
        [
          "exec",
          "-T",
          "postgres",
          "psql",
          "-U",
          "analytics",
          "-d",
          "analytics",
          "-At",
          "-v",
          "ON_ERROR_STOP=1",
          "-c",
          "SELECT COUNT(*) FROM web_vital_facts WHERE site_id='site_playground'",
        ],
        { capture: true },
      ).trim(),
    );
    assert(factCount > 0, "Processor did not persist browser Web Vital facts");

    const reportDate = new Date(pageViews.get(vitals[0].page_view_event_id).occurred_at)
      .toISOString()
      .slice(0, 10);
    const reportResponse = await fetch(
      `${analyticsUrl}/v1/sites/site_playground/reports/${reportDate}/${reportDate}/web-vitals`,
    );
    const report = await reportResponse.json();
    assert(
      reportResponse.status === 200,
      `Analytics API rejected browser Web Vitals: ${JSON.stringify(report)}`,
    );
    assert(
      report.total === factCount &&
        report.items.reduce((sum, item) => sum + item.count, 0) === factCount,
      "Analytics API total does not include all processed browser samples",
    );
    assert(
      vitals.every((event) =>
        report.items.some((item) => item.path === event.path && item.metric === event.metric),
      ),
      "Analytics API is missing browser generated Web Vital metrics",
    );
  } finally {
    await context.close();
  }
}

function readBrowserEvents() {
  const output = runCompose(
    [
      "exec",
      "-T",
      "postgres",
      "psql",
      "-U",
      "analytics",
      "-d",
      "analytics",
      "-At",
      "-v",
      "ON_ERROR_STOP=1",
      "-c",
      "SELECT COALESCE(json_agg(payload ORDER BY id), '[]'::json)::text FROM raw_events WHERE site_id='site_playground'",
    ],
    { capture: true },
  );
  return JSON.parse(output.trim());
}

async function assertWebVitals(page) {
  const data = await fixture("web-vitals");
  await prepareFixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-18", "2026-09-18"));
  const section = page.locator("section.card").filter({ hasText: "Web Vitals" });
  await expectReportRows(page, "Web Vitals", [
    "/vitals CLS 4 0.09 3 1 0",
    "/vitals FCP 4 1800 ms 3 1 0",
    "/vitals INP 4 200 ms 3 0 1",
    "/vitals LCP 4 2500 ms 3 0 1",
    "/vitals TTFB 4 800 ms 3 0 1",
  ]);
  assert(
    !(await section.textContent()).includes("properties"),
    "Web Vitals must not display properties",
  );
}

async function assertCustomEvents(page) {
  const data = await fixture("custom-events");
  await prepareFixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-18", "2026-09-18"));
  const section = page.locator("section.card").filter({ hasText: "Custom Events" });
  await section.getByText("2", { exact: true }).waitFor();
  await expectReportRows(page, "Custom Events", [
    "checkout_started 2026-09-18 1",
    "purchase_completed 2026-09-18 1",
  ]);
  const content = await section.textContent();
  assert(
    !content.includes("amount") && !content.includes("email"),
    "Custom event properties must not be displayed",
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
  await page
    .locator("section.card")
    .filter({ hasText: "Web Vitals" })
    .getByText("No page view data is available for this selection.")
    .waitFor();
  const emptyCopy = "No page view data is available for this selection.";
  assert(
    (await page
      .locator("section.card")
      .filter({ hasText: "Timeline" })
      .getByText(emptyCopy)
      .count()) === 1 &&
      (await page
        .locator("section.card")
        .filter({ hasText: "Top Pages" })
        .getByText(emptyCopy)
        .count()) === 1,
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

async function assertPhase6Dashboard(page) {
  const data = await phase6Fixture("dashboard-dimensions");
  await preparePhase6Fixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-20", "2026-09-21"));
  await expectMetric(page, "Unique Visitors", data.expected.visitors.unique_visitors);
  await expectMetric(page, "Sessions", data.expected.visitors.sessions);
  await expectReportRows(
    page,
    "Visitors and Sessions",
    data.expected.visitors.items.map(
      (item) => `${item.day} ${item.page_views} ${item.unique_visitors} ${item.sessions}`,
    ),
  );
  await expectReportRows(page, "Dimension Report", [
    `${data.expected.browser.value} ${data.expected.browser.page_views} ${data.expected.browser.unique_visitors} ${data.expected.browser.sessions}`,
  ]);
  await page.locator(".freshness-current").waitFor();
  await page.locator('select[name="dimension"]').selectOption("language");
  await page.locator('button[type="submit"]').click();
  await page.waitForURL(/dimension=language/);
  await expectReportRows(page, "Dimension Report", ["en-CA 3 1 2", "en-US 2 1 1"]);
}

async function assertPhase6Disabled(page) {
  const data = await fixture("single-page-view");
  await prepareFixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-18", "2026-09-18"));
  await expectMetric(
    page,
    "Selected range Page Views",
    data.expected.api.range_overview.body.page_views,
  );
  assert(
    (await page.getByText("Phase 6 analytics is not enabled for this site.").count()) === 2,
    "Expected disabled state for Visitors and Dimensions",
  );
}

async function assertPhase6Empty(page) {
  const data = await phase6Fixture("dashboard-dimensions");
  await preparePhase6Fixture(data);
  await page.goto(rangeUrl("site_playground", "2026-09-01", "2026-09-01"));
  assert(
    (await page.getByText("No Phase 6 analytics data is available for this selection.").count()) ===
      2,
    "Expected empty state for Visitors and Dimensions",
  );
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
  const artifactDirectory = path.join(root, "artifacts", "dashboard-e2e");
  await rm(artifactDirectory, { recursive: true, force: true });
  assertDashboardIsolation();
  const composeOutput = runCompose(
    [
      "up",
      "-d",
      "--build",
      "--wait",
      "postgres",
      "collector",
      "analytics-api",
      "dashboard",
      "playground-next",
    ],
    {
      capture: true,
      env: {
        NEXT_PUBLIC_ANALYTICS_TRANSPORT: "fetch",
        NEXT_PUBLIC_ANALYTICS_ENDPOINT: `${collectorUrl}/v1/events`,
        NEXT_PUBLIC_ANALYTICS_INGEST_KEY: keys.site_playground,
        NEXT_PUBLIC_ANALYTICS_SITE_ID: "site_playground",
      },
    },
  );
  process.stdout.write(composeOutput);
  await waitFor("Dashboard", `${dashboardUrl}/dashboard`);
  await waitFor("Next.js browser playground", "http://localhost:3000");
  await assertDashboardRuntimeConfiguration();
  browser = await chromium.launch({ headless: true });
  await assertBrowserWebVitalsCollection(browser);
  console.log("PASS real-browser Web Vitals collection and keepalive ingestion");
  browserContext = await browser.newContext();
  await browserContext.tracing.start({ screenshots: true, snapshots: true });
  page = await browserContext.newPage();

  await assertSinglePageView(page);
  console.log("PASS single-page-view dashboard");
  await assertCustomEvents(page);
  console.log("PASS custom-events dashboard");
  await assertWebVitals(page);
  console.log("PASS web-vitals dashboard");
  await assertMultiPageNavigation(page);
  console.log("PASS multi-page-navigation dashboard");
  await assertMultiSiteIsolation(page);
  console.log("PASS multi-site-isolation dashboard");
  await assertEmptyRange(page);
  console.log("PASS empty-date-range dashboard");
  await assertCustomDateRange(page);
  console.log("PASS custom-date-range dashboard");
  await assertPhase6Disabled(page);
  console.log("PASS phase6-disabled dashboard");
  await assertPhase6Dashboard(page);
  console.log("PASS phase6 dashboard");
  await assertPhase6Empty(page);
  console.log("PASS phase6-empty dashboard");
  await assertApiError(browser);
  console.log("PASS api-error dashboard");
  await page.close();
  console.log("Dashboard E2E workflow passed.");
} catch (error) {
  const artifactDirectory = path.join(root, "artifacts", "dashboard-e2e");
  await mkdir(artifactDirectory, { recursive: true });
  if (page) {
    try {
      await page.screenshot({ path: path.join(artifactDirectory, "failure.png"), fullPage: true });
    } catch (artifactError) {
      console.error(`Dashboard failure screenshot could not be saved: ${artifactError}`);
    }
  }
  if (browserContext) {
    try {
      await browserContext.tracing.stop({ path: path.join(artifactDirectory, "trace.zip") });
    } catch (artifactError) {
      console.error(`Dashboard trace could not be saved: ${artifactError}`);
    }
  }
  try {
    await writeFile(
      path.join(artifactDirectory, "compose-config.txt"),
      runCompose(["config"], { capture: true, allowFailure: true }),
    );
    await writeFile(
      path.join(artifactDirectory, "compose-ps.txt"),
      runCompose(["ps", "--all"], { capture: true, allowFailure: true }),
    );
    await writeFile(
      path.join(artifactDirectory, "service-logs.txt"),
      runCompose(
        [
          "logs",
          "--no-color",
          "dashboard",
          "playground-next",
          "collector",
          "analytics-api",
          "postgres",
          "db-migrate",
        ],
        {
          capture: true,
          allowFailure: true,
        },
      ),
    );
    console.error(`Dashboard E2E artifacts saved in ${artifactDirectory}`);
  } catch (artifactError) {
    console.error(`Dashboard Compose diagnostics could not be saved: ${artifactError}`);
  }
  console.error(error instanceof Error ? (error.stack ?? error.message) : String(error));
  process.exitCode = 1;
} finally {
  if (browserContext) await browserContext.close();
  if (browser) await browser.close();
  runCompose(["down", "--volumes", "--remove-orphans"], { allowFailure: true });
}
