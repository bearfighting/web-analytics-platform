import { execFileSync } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const composeFiles = ["-f", "compose.yaml", "-f", "compose.backend.yaml", "-f", "compose.e2e.yaml"];
const project = `web-analytics-e2e-${process.pid}`;
const collectorUrl = `http://127.0.0.1:${process.env.E2E_COLLECTOR_PORT ?? "14001"}`;
const analyticsUrl = `http://127.0.0.1:${process.env.E2E_ANALYTICS_API_PORT ?? "14002"}`;
const origin = "http://localhost:3000";
const fixturesDirectory = path.join(
  root,
  "protocol",
  "contracts",
  "analytics-api",
  "current",
  "fixtures",
);
const fixtureNames = [
  "single-page-view.json",
  "multi-page-navigation.json",
  "duplicate-events.json",
  "late-event.json",
  "multi-site-isolation.json",
  "empty-date-range.json",
  "all-time-overview.json",
  "custom-events.json",
].filter((name) => !process.env.E2E_FIXTURES || process.env.E2E_FIXTURES.split(",").includes(name));

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
];
let processorOutput = "";

try {
  await runCompose(["up", "-d", "--build", "--wait", "postgres", "collector", "analytics-api"]);
  await waitFor("Analytics API", `${analyticsUrl}/health`, (body) => body.status === "ok");
  await runFixtures();
  console.log(`E2E analytics workflow passed for ${fixtureNames.length} fixtures.`);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  await writeDiagnostics();
  process.exitCode = 1;
} finally {
  runCompose(["down", "--volumes", "--remove-orphans"], { allowFailure: true });
}

async function runFixtures() {
  for (const fixtureName of fixtureNames) {
    const fixture = JSON.parse(await readFile(path.join(fixturesDirectory, fixtureName), "utf8"));
    await resetDatabase();
    const startedAt = new Date();

    if (fixture.input.events.length > 0) {
      const keys = new Map([
        ["site_playground", "e2e-test-key"],
        ["site_alpha", "e2e-test-key-alpha"],
        ["site_beta", "e2e-test-key-beta"],
      ]);
      for (const siteId of new Set(fixture.input.events.map((event) => event.site_id))) {
        await postEvents(
          fixture.input.events.filter((event) => event.site_id === siteId),
          keys.get(siteId),
        );
      }
    }

    const finishedAt = new Date();
    await runProcessorOnce();
    await assertFixture(fixture, startedAt, finishedAt);

    if (fixture.input.events.length > 0) {
      for (const siteId of new Set(fixture.input.events.map((event) => event.site_id))) {
        const keys = {
          site_playground: "e2e-test-key",
          site_alpha: "e2e-test-key-alpha",
          site_beta: "e2e-test-key-beta",
        };
        await postEvents(
          fixture.input.events.filter((event) => event.site_id === siteId),
          keys[siteId],
        );
      }
      await runProcessorOnce();
      await assertFixture(fixture, startedAt, finishedAt, true);
    }

    if (fixture.id === "custom-events") {
      runCustomEventRebuild();
      await assertFixture(fixture, startedAt, finishedAt);
    }

    console.log(`PASS ${fixture.id}`);
  }
}

async function postEvents(events, ingestKey) {
  const response = await fetch(`${collectorUrl}/v1/events`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      origin,
      "x-ingest-key": ingestKey,
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

async function runProcessorOnce() {
  try {
    processorOutput += runCompose(
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
  } catch (error) {
    const cause = error.cause ?? error;
    processorOutput += [cause.stdout, cause.stderr].filter(Boolean).join("\n");
    throw error;
  }
}

function runCustomEventRebuild() {
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
      "--rebuild-custom-events",
      "--site-id",
      "site_playground",
    ],
    { capture: true },
  );
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
    "TRUNCATE analytics_rebuild_queue, dimension_event_facts, dimension_daily, normalized_event_context, session_events, sessions, visitor_event_facts, session_daily, visitor_daily, analytics_watermarks, analytics_generations, analytics_feature_flags, raw_events, page_view_daily, page_view_routes, page_view_totals RESTART IDENTITY CASCADE",
  ]);
}

async function assertFixture(fixture, startedAt, finishedAt, repeated = false) {
  const expected = fixture.expected;
  const raw = queryJson(
    "SELECT site_id, event_id, schema_version, event_type, (extract(epoch FROM occurred_at) * 1000)::bigint AS occurred_at, path, payload, processed_at IS NOT NULL AS processed FROM raw_events ORDER BY id",
  );
  const expectedRaw = expected.raw_events.events.map((event) => ({
    site_id: event.site_id,
    event_id: event.event_id,
    schema_version: event.schema_version,
    event_type: event.type,
    occurred_at: event.occurred_at,
    path: event.path ?? null,
    payload: event.payload,
    processed: true,
  }));
  assertJsonEqual(raw, expectedRaw, `${fixture.id}: raw_events mismatch`);
  assert(
    fixture.input.events.length - raw.length === expected.raw_events.duplicates_ignored,
    `${fixture.id}: duplicate count mismatch`,
  );
  assert(
    raw.every((event) => event.processed),
    `${fixture.id}: unprocessed raw event remains`,
  );

  const daily = queryJson(
    "SELECT site_id, day::text AS day, page_views FROM page_view_daily ORDER BY site_id, day",
  );
  const routes = queryJson(
    "SELECT site_id, day::text AS day, path, page_views FROM page_view_routes ORDER BY site_id, day, path",
  );
  const totals = queryJson("SELECT site_id, page_views FROM page_view_totals ORDER BY site_id");
  assertJsonEqual(daily, expected.page_view_daily, `${fixture.id}: daily aggregate mismatch`);
  assertJsonEqual(
    routes,
    sortRecords(expected.page_view_routes, ["site_id", "day", "path"]),
    `${fixture.id}: route aggregate mismatch`,
  );
  assertJsonEqual(totals, expected.page_view_totals, `${fixture.id}: total aggregate mismatch`);

  const receivedCount = queryJson(
    `SELECT COUNT(*)::int AS count FROM raw_events WHERE received_at >= '${startedAt.toISOString()}' AND received_at <= '${finishedAt.toISOString()}'`,
  )[0].count;
  assert(
    receivedCount === expected.raw_events.inserted,
    `${fixture.id}: received_at count mismatch`,
  );

  if (repeated) return;
  await assertApiResponses(expected.api);
}

async function assertApiResponses(api) {
  await assertApi(`/v1/sites/${api.overview.body.site_id}/overview`, api.overview);
  await assertApi(
    `/v1/sites/${api.range_overview.body.site_id}/reports/${api.range_overview.body.from}/${api.range_overview.body.to}/overview`,
    api.range_overview,
  );
  await assertApi(
    `/v1/sites/${api.timeline.body.site_id}/reports/${api.timeline.body.from}/${api.timeline.body.to}/timeline`,
    api.timeline,
  );
  await assertApi(
    `/v1/sites/${api.pages.body.site_id}/reports/${api.pages.body.from}/${api.pages.body.to}/pages`,
    api.pages,
  );
  if (api.events) {
    const expected = structuredClone(api.events);
    expected.body.data_as_of = undefined;
    const response = await fetch(
      `${analyticsUrl}/v1/sites/${api.events.body.site_id}/reports/${api.events.body.from}/${api.events.body.to}/events`,
    );
    const body = await response.json();
    assert(
      response.status === expected.status,
      `events report expected HTTP ${expected.status}, got ${response.status}`,
    );
    assert(
      body.data_as_of !== null,
      "events report must expose its independent processed watermark",
    );
    delete body.data_as_of;
    assertJsonEqual(body, expected.body, "events report response mismatch");

    const filteredResponse = await fetch(
      `${analyticsUrl}/v1/sites/${api.events.body.site_id}/reports/${api.events.body.from}/${api.events.body.to}/events?event_name=checkout_started&limit=1`,
    );
    const filtered = await filteredResponse.json();
    assert(
      filteredResponse.status === 200 &&
        filtered.total === 1 &&
        filtered.items.length === 1 &&
        filtered.items[0].event_name === "checkout_started",
      "event_name filter must match exactly and retain its complete filtered total",
    );

    const isolatedResponse = await fetch(
      `${analyticsUrl}/v1/sites/site_alpha/reports/${api.events.body.from}/${api.events.body.to}/events`,
    );
    const isolated = await isolatedResponse.json();
    assert(
      isolatedResponse.status === 200 &&
        isolated.total === 0 &&
        isolated.items.length === 0 &&
        isolated.data_as_of === null &&
        isolated.freshness_status === "current",
      "empty events reports must remain site-isolated and expose freshness",
    );
  }
  if (api.site_beta_overview)
    await assertApi(
      `/v1/sites/${api.site_beta_overview.body.site_id}/overview`,
      api.site_beta_overview,
    );
  if (api.unknown_site) {
    for (const [name, expected] of Object.entries(api.unknown_site)) {
      const body = expected.body;
      const reportEndpoint = name === "range_overview" ? "overview" : name;
      const pathName =
        name === "overview"
          ? `/v1/sites/${body.site_id}/overview`
          : `/v1/sites/${body.site_id}/reports/${body.from}/${body.to}/${reportEndpoint}`;
      await assertApi(pathName, expected);
    }
  }
}

async function assertApi(pathName, expected) {
  const response = await fetch(`${analyticsUrl}${pathName}`);
  const body = await response.json();
  assert(
    response.status === expected.status,
    `${pathName}: expected HTTP ${expected.status}, got ${response.status}`,
  );
  assertJsonEqual(body, expected.body, `${pathName}: response mismatch`);
}

function queryJson(sql) {
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
      "-v",
      "ON_ERROR_STOP=1",
      "-At",
      "-c",
      `SELECT COALESCE(json_agg(row_to_json(result)), '[]'::json) FROM (${sql}) AS result`,
    ],
    { capture: true },
  );
  return JSON.parse(output.trim() || "[]");
}

async function waitFor(label, url, predicate) {
  const deadline = Date.now() + 300_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      const body = await response.json();
      if (predicate(body)) return;
    } catch {
      // The service may still be compiling or starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`${label} did not become ready at ${url}`);
}

function runCompose(args, options = {}) {
  try {
    return execFileSync("docker", [...composeBaseArgs, ...args], {
      cwd: root,
      encoding: "utf8",
      stdio: options.capture ? ["ignore", "pipe", "pipe"] : "inherit",
    });
  } catch (error) {
    if (options.allowFailure) {
      return [error.stdout, error.stderr].filter(Boolean).join("\n");
    }
    throw new Error(`docker compose ${args.join(" ")} failed`);
  }
}

async function writeDiagnostics() {
  const artifactDirectory = path.join(root, "artifacts", "analytics-e2e");
  await mkdir(artifactDirectory, { recursive: true });
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
    runCompose(["logs", "--no-color", "collector", "processor", "analytics-api", "db-migrate"], {
      capture: true,
      allowFailure: true,
    }),
  );
  await writeFile(path.join(artifactDirectory, "processor-output.txt"), processorOutput);
  console.error(`E2E analytics diagnostics saved in ${artifactDirectory}`);
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertJsonEqual(actual, expected, message) {
  assert(
    JSON.stringify(sortJson(actual)) === JSON.stringify(sortJson(expected)),
    `${message}\nactual=${JSON.stringify(actual)}\nexpected=${JSON.stringify(expected)}`,
  );
}

function sortJson(value) {
  if (Array.isArray(value)) return value.map(sortJson);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, item]) => [key, sortJson(item)]),
    );
  }
  return value;
}

function sortRecords(records, keys) {
  return [...records].sort((left, right) => {
    for (const key of keys) {
      const comparison = String(left[key]).localeCompare(String(right[key]));
      if (comparison !== 0) return comparison;
    }
    return 0;
  });
}
