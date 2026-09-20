import { readFile } from "node:fs/promises";
import { readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const contractRoot = resolve(root, "protocol/phase-5/contract");
const schemaRoot = resolve(contractRoot, "schemas");
const fixtureRoot = resolve(contractRoot, "../fixtures");

const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);

const contextSchema = await readJson(resolve(schemaRoot, "browser-context-v1.schema.json"));
const eventSchema = await readJson(resolve(schemaRoot, "page-view-event-v2.schema.json"));
const batchSchema = await readJson(resolve(schemaRoot, "event-batch-v2.schema.json"));
const v1EventSchema = await readJson(resolve(root, "protocol/schemas/page-view-event.schema.json"));
ajv.addSchema(contextSchema);
ajv.addSchema(eventSchema);
ajv.addSchema(v1EventSchema);
const validateEvent = ajv.compile(eventSchema);
const validateBatch = ajv.compile(batchSchema);
const validateV1Event = ajv.compile(v1EventSchema);
const errors = [];

function assert(condition, message) {
  if (!condition) errors.push(message);
}

async function validateSchemaFixtures(directory, expectedValid, label) {
  for (const filename of readdirSync(directory)
    .filter((name) => name.endsWith(".json"))
    .sort()) {
    const path = resolve(directory, filename);
    const fixture = await readJson(path);
    const validator = filename === "event-batch-v2.json" ? validateBatch : validateEvent;
    const valid = validator(fixture);
    assert(valid === expectedValid, `${label}/${filename}: unexpected schema result`);
    if (valid !== expectedValid) console.error(validator.errors ?? "no validation details");
  }
}

await validateSchemaFixtures(resolve(fixtureRoot, "valid"), true, "valid");
await validateSchemaFixtures(resolve(fixtureRoot, "invalid"), false, "invalid");

const v1Compatibility = await readJson(resolve(contractRoot, "../examples/v1-compatibility.json"));
assert(
  validateV1Event(v1Compatibility.v1_event),
  "V1 compatibility event must pass the active V1 schema",
);
assert(v1Compatibility.v1_event.schema_version === 1, "V1 compatibility fixture must remain V1");
assert(v1Compatibility.expected.page_views === 1, "V1 compatibility must preserve Page Views");
assert(
  v1Compatibility.expected.visitor_session_eligible === false,
  "V1 compatibility must lack identity eligibility",
);
assert(
  v1Compatibility.expected.shared_fallback_identity === false,
  "V1 compatibility must not use a shared fallback",
);

const v2Migration = await readJson(resolve(contractRoot, "../examples/v2-migration.json"));
assert(
  validateV1Event(v2Migration.v1_event),
  "V2 migration V1 event must pass the active V1 schema",
);
assert(validateEvent(v2Migration.v2_event), "V2 migration event must pass the V2 schema");
assert(
  v2Migration.expected.v1_page_view_semantics_unchanged === true,
  "V2 migration must preserve V1 Page View semantics",
);
assert(
  v2Migration.expected.identified_page_views === 1,
  "V2 migration must identify only the V2 event",
);

const semantic = await readJson(resolve(fixtureRoot, "semantic/cases.json"));
assert(semantic.schema_versions.event_v1 === 1, "semantic fixture must declare Event V1 version");
assert(semantic.schema_versions.event_v2 === 2, "semantic fixture must declare Event V2 version");
assert(semantic.schema_versions.context === 1, "semantic fixture must declare Context V1 version");
assert(
  semantic.parser_version === "deferred-to-phase-6",
  "parser version must remain explicitly deferred",
);
const semanticIds = new Set();
const ingestedEventsByScenario = new Map();
const eventIdPattern = /^[0-9A-HJKMNP-TV-Z]{26}$/;

function countValues(events, selector) {
  return new Set(events.map(selector)).size;
}

function assertReceivedOrder(scenario) {
  const expected = [...scenario.events].map((event) => event.event_id).sort();
  const actual = [...scenario.received_order].sort();
  assert(
    JSON.stringify(actual) === JSON.stringify(expected),
    `${scenario.id}: received_order must be a permutation of event IDs`,
  );
}

function ingestByReceivedOrder(scenario) {
  const pendingById = new Map();
  for (const event of scenario.events) {
    const pending = pendingById.get(event.event_id) ?? [];
    pending.push(event);
    pendingById.set(event.event_id, pending);
  }

  const accepted = [];
  const seen = new Set();
  for (const eventId of scenario.received_order) {
    const event = pendingById.get(eventId)?.shift();
    if (!event) {
      assert(false, `${scenario.id}: received_order references an unknown event`);
      continue;
    }
    const dedupeKey = `${event.site_id}:${event.event_id}`;
    if (seen.has(dedupeKey)) continue;
    seen.add(dedupeKey);
    if (
      typeof event.received_at === "number" &&
      event.occurred_at - event.received_at > 5 * 60 * 1000
    )
      continue;
    accepted.push(event);
  }
  return accepted;
}

function countSessions(events) {
  const groups = new Map();
  for (const event of events) {
    if (!event.visitor_id) continue;
    const key = `${event.site_id}:${event.visitor_id}`;
    const group = groups.get(key) ?? [];
    group.push(event);
    groups.set(key, group);
  }

  let sessions = 0;
  for (const group of groups.values()) {
    group.sort(
      (left, right) =>
        left.occurred_at - right.occurred_at || left.event_id.localeCompare(right.event_id),
    );
    let previous;
    for (const event of group) {
      if (!previous) {
        sessions += 1;
      } else {
        const gap = event.occurred_at - previous.occurred_at;
        const previousDay = new Date(previous.occurred_at).toISOString().slice(0, 10);
        const currentDay = new Date(event.occurred_at).toISOString().slice(0, 10);
        if (gap >= 30 * 60 * 1000 || previousDay !== currentDay) sessions += 1;
      }
      previous = event;
    }
  }
  return sessions;
}

function normalizeContext(raw) {
  const dimension = (value) =>
    Number.isInteger(value) && value >= 0 && value <= 100000 ? value : "unknown";
  const languagePattern = /^[A-Za-z]{2,8}(?:-[A-Za-z0-9]{1,8})*$/;
  const knownTimezones = new Set(Intl.supportedValuesOf("timeZone"));
  const textOrUnknown = (value, maxLength, isValid = () => true) =>
    typeof value === "string" && value.length > 0 && value.length <= maxLength && isValid(value)
      ? value
      : "unknown";
  const normalized = {
    language: textOrUnknown(raw.language, 64, (value) => languagePattern.test(value)),
    timezone: textOrUnknown(raw.timezone, 64, (value) => knownTimezones.has(value)),
    viewport_width: dimension(raw.viewport_width),
    viewport_height: dimension(raw.viewport_height),
    screen_width: dimension(raw.screen_width),
    screen_height: dimension(raw.screen_height),
    user_agent: textOrUnknown(raw.user_agent, 1024),
  };
  const textLimits = {
    utm_source: 256,
    utm_medium: 256,
    utm_campaign: 256,
    utm_term: 256,
    utm_content: 256,
    referrer: 4096,
  };
  for (const [field, maxLength] of Object.entries(textLimits)) {
    if (typeof raw[field] === "string" && raw[field].length > 0 && raw[field].length <= maxLength) {
      normalized[field] = raw[field];
    }
  }
  return normalized;
}

for (const scenario of semantic.cases) {
  assert(!semanticIds.has(scenario.id), `semantic fixture has duplicate id: ${scenario.id}`);
  semanticIds.add(scenario.id);
  assert(Array.isArray(scenario.events), `${scenario.id}: events must be an array`);
  assert(Array.isArray(scenario.received_order), `${scenario.id}: received_order must be an array`);
  assert(
    scenario.events.length === scenario.received_order.length,
    `${scenario.id}: received_order must cover all inputs`,
  );
  assert(
    scenario.events.every((event) => typeof event.event_id === "string"),
    `${scenario.id}: every event needs event_id`,
  );
  assert(
    scenario.events.every((event) => eventIdPattern.test(event.event_id)),
    `${scenario.id}: every event_id must be a ULID`,
  );
  assert(
    scenario.events.every((event) => typeof event.occurred_at === "number"),
    `${scenario.id}: every event needs occurred_at`,
  );
  assert(
    scenario.versions?.event === scenario.schema_version,
    `${scenario.id}: versions.event must match schema_version`,
  );
  assert(
    scenario.versions?.context === null || scenario.versions?.context === 1,
    `${scenario.id}: versions.context must be null or 1`,
  );
  assert(
    typeof scenario.versions?.parser === "string",
    `${scenario.id}: versions.parser is required`,
  );
  const expected = scenario.expected;
  for (const field of ["page_views", "visitor_session_eligible", "unique_visitors", "sessions"]) {
    assert(
      Number.isInteger(expected?.[field]) && expected[field] >= 0,
      `${scenario.id}: expected.${field} is required`,
    );
  }
  assert(
    expected?.dedupe_key === "site_id+event_id",
    `${scenario.id}: expected.dedupe_key is required`,
  );
  assert(
    typeof expected?.late_event_handling === "string",
    `${scenario.id}: expected.late_event_handling is required`,
  );
  assert(
    Array.isArray(expected?.derived_dimensions?.allowed),
    `${scenario.id}: derived_dimensions.allowed is required`,
  );
  assert(
    Array.isArray(expected?.derived_dimensions?.forbidden),
    `${scenario.id}: derived_dimensions.forbidden is required`,
  );
  assertReceivedOrder(scenario);
  ingestedEventsByScenario.set(scenario.id, ingestByReceivedOrder(scenario));
  const ingested = ingestedEventsByScenario.get(scenario.id);
  const eligible = ingested.filter((event) => typeof event.visitor_id === "string");
  assert(
    expected.page_views === ingested.length,
    `${scenario.id}: expected.page_views does not match ingestion`,
  );
  assert(
    expected.visitor_session_eligible === eligible.length,
    `${scenario.id}: expected.visitor_session_eligible does not match ingestion`,
  );
  assert(
    expected.unique_visitors ===
      new Set(eligible.map((event) => `${event.site_id}:${event.visitor_id}`)).size,
    `${scenario.id}: expected.unique_visitors does not match ingestion`,
  );
  assert(
    expected.sessions === countSessions(ingested),
    `${scenario.id}: expected.sessions does not match sessionization`,
  );
}

const byId = (id) => semantic.cases.find((scenario) => scenario.id === id);
const timeout = byId("session-timeout-boundaries");
assert(
  timeout.expected.session_timeout_ms === 30 * 60 * 1000,
  "session timeout must be 30 minutes",
);
assert(
  timeout.expected.new_session_at_or_after_timeout === true,
  "30-minute boundary must start a new Session",
);
assert(
  countSessions(ingestedEventsByScenario.get(timeout.id)) === timeout.expected.sessions,
  "timeout fixture session count must match its events",
);
assert(
  countSessions(ingestedEventsByScenario.get("single-visitor-session")) === 1,
  "single visitor fixture must produce one Session",
);
assert(
  countSessions(ingestedEventsByScenario.get("midnight-split")) === 2,
  "midnight fixture must produce two Sessions",
);
assert(
  countSessions(ingestedEventsByScenario.get("multi-tab-shared-visitor")) === 1,
  "multi-tab fixture must produce one Session",
);
const duplicateLate = byId("duplicate-and-late-events");
const ingestedDuplicateLate = ingestedEventsByScenario.get(duplicateLate.id);
const uniqueEventKeys = new Set(
  ingestedDuplicateLate.map((event) => `${event.site_id}:${event.event_id}`),
);
assert(
  ingestedDuplicateLate.length === duplicateLate.expected.unique_events &&
    uniqueEventKeys.size === duplicateLate.expected.unique_events,
  "duplicate fixture must deduplicate by site and event ID",
);
assert(
  duplicateLate.expected.dedupe_key === "site_id+event_id",
  "duplicates must use site_id + event_id",
);
assert(
  duplicateLate.expected.session_ordering === "occurred_at",
  "Session ordering must use occurred_at",
);
const receivedUniqueOrder = ingestedDuplicateLate.map((event) => event.event_id);
const occurredOrder = [...ingestedDuplicateLate]
  .sort(
    (left, right) =>
      left.occurred_at - right.occurred_at || left.event_id.localeCompare(right.event_id),
  )
  .map((event) => event.event_id);
assert(
  receivedUniqueOrder.join(",") !== occurredOrder.join(","),
  "late-event fixture must distinguish received order from occurred_at order",
);
assert(
  countSessions(ingestedDuplicateLate) === duplicateLate.expected.reconstructed_sessions,
  "late events must be sessionized from occurred_at order after ingestion",
);
assert(
  duplicateLate.expected.automatic_rebuild_within_ms === 24 * 60 * 60 * 1000,
  "automatic lateness window must be 24 hours",
);
const context = byId("browser-context-normalization");
const { ip_omitted, utm_source_omitted, ...expectedContext } = context.expected_normalized_context;
assert(
  JSON.stringify(normalizeContext(context.raw_context)) === JSON.stringify(expectedContext),
  "context normalization must match expected output",
);
const boundaryContext = normalizeContext({
  language: "invalid language",
  timezone: "Not/AZone",
  viewport_width: 1440,
  viewport_height: 900,
  screen_width: 2560,
  screen_height: 1440,
  user_agent: "u".repeat(1025),
  utm_source: "s".repeat(257),
  referrer: "r".repeat(4097),
});
assert(boundaryContext.language === "unknown", "invalid language must normalize to unknown");
assert(boundaryContext.timezone === "unknown", "invalid timezone must normalize to unknown");
assert(boundaryContext.user_agent === "unknown", "overlong user-agent must normalize to unknown");
assert(!("utm_source" in boundaryContext), "overlong UTM must be omitted");
assert(!("referrer" in boundaryContext), "overlong referrer must be omitted");
assert(ip_omitted === true, "IP must be omitted from normalized context");
assert(utm_source_omitted === true, "empty UTM must be omitted");
const future = byId("future-event-boundary");
const futureLeads = future.events.map((event) => event.occurred_at - event.received_at);
assert(futureLeads[0] === 5 * 60 * 1000, "future fixture must include the exact 5-minute boundary");
assert(
  futureLeads[1] > 5 * 60 * 1000,
  "future fixture must include a value beyond the allowed boundary",
);
assert(
  future.expected.max_client_clock_lead_ms === 5 * 60 * 1000,
  "future event tolerance must be 5 minutes",
);
assert(future.expected.five_minute_boundary_allowed === true, "5-minute boundary must be allowed");
assert(
  future.expected.over_boundary_rejected === true,
  "future events beyond 5 minutes must be rejected",
);
const siteIsolation = byId("site-isolated-visitor");
assert(
  countValues(siteIsolation.events, (event) => `${event.site_id}:${event.visitor_id}`) === 2,
  "Visitor identity must be counted per site",
);
assert(siteIsolation.expected.site_scope_isolation === true, "Visitor IDs must be site-scoped");
assert(
  siteIsolation.expected.cross_site_identity_link === false,
  "Visitor IDs must not link sites",
);
const privacy = byId("privacy-boundary");
assert(
  privacy.expected.forbidden_fields_are_not_contract_fields === true,
  "privacy fixture must forbid sensitive fields",
);
assert(
  privacy.expected.visitor_identity_source === "visitor_id_only",
  "Visitor identity must come only from visitor_id",
);
for (const field of privacy.forbidden_fields) {
  assert(
    !Object.hasOwn(contextSchema.properties, field),
    `privacy fixture field '${field}' must not be a Browser Context contract field`,
  );
}

if (errors.length > 0) {
  for (const error of errors) console.error(`FAIL ${error}`);
  process.exitCode = 1;
} else {
  console.log(`Phase 5 contract validated: ${semantic.cases.length} semantic cases.`);
}
