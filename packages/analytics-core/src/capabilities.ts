export const CAPABILITY_IDS = [
  "page_views",
  "browser_context",
  "anonymous_visitors",
  "sessions",
  "dimensions",
  "custom_events",
  "web_vitals",
  "conversions",
  "funnels",
  "geo",
] as const;

export type CapabilityId = (typeof CAPABILITY_IDS)[number];
export type CapabilityStatus = "implemented" | "planned";
export type CapabilityRebuild = "none" | "incremental" | "explicit_backfill";

export interface CapabilityContract {
  id: CapabilityId;
  status: CapabilityStatus;
  depends_on: readonly CapabilityId[];
  input: { readonly event_types: readonly string[]; readonly required_fields: readonly string[] };
  raw_event: { readonly preserve: boolean; readonly fields: readonly string[] };
  processor: { readonly facts: readonly string[]; readonly rebuild: CapabilityRebuild };
  api: {
    readonly query_surface: "overview" | "report" | "future_report" | "none";
    readonly routes: readonly string[];
  };
  dashboard: {
    readonly surface: "overview" | "report" | "future_report" | "none";
    readonly disabled: string;
    readonly empty: string;
    readonly error: string;
  };
  security: {
    readonly consent: "required" | "not_applicable";
    readonly origin_and_ingest_key: "enforced" | "not_applicable";
    readonly privacy: readonly string[];
  };
  history: {
    readonly queryable_after_disabled: boolean;
    readonly backfill: "never" | "optional" | "required";
  };
}

export interface CapabilityManifest {
  capability_schema_version: 1;
  capabilities: readonly CapabilityContract[];
}

export function createCapabilityRegistry(manifest: CapabilityManifest) {
  const contracts = new Map(
    manifest.capabilities.map((capability) => [capability.id, freezeCapability(capability)]),
  );

  return {
    get(id: CapabilityId): Readonly<CapabilityContract> | undefined {
      return contracts.get(id);
    },
    dependencies(id: CapabilityId): readonly CapabilityId[] {
      return contracts.get(id)?.depends_on ?? [];
    },
    isImplemented(id: CapabilityId): boolean {
      return contracts.get(id)?.status === "implemented";
    },
  };
}

function freezeCapability(capability: CapabilityContract): Readonly<CapabilityContract> {
  return Object.freeze({
    ...capability,
    depends_on: Object.freeze([...capability.depends_on]),
    input: Object.freeze({
      ...capability.input,
      event_types: Object.freeze([...capability.input.event_types]),
      required_fields: Object.freeze([...capability.input.required_fields]),
    }),
    raw_event: Object.freeze({
      ...capability.raw_event,
      fields: Object.freeze([...capability.raw_event.fields]),
    }),
    processor: Object.freeze({
      ...capability.processor,
      facts: Object.freeze([...capability.processor.facts]),
    }),
    api: Object.freeze({ ...capability.api, routes: Object.freeze([...capability.api.routes]) }),
    dashboard: Object.freeze({ ...capability.dashboard }),
    security: Object.freeze({
      ...capability.security,
      privacy: Object.freeze([...capability.security.privacy]),
    }),
    history: Object.freeze({ ...capability.history }),
  });
}
