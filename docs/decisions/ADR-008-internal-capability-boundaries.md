# ADR-008：Internal Capability Boundaries

- Status: Accepted
- Date: 2026-09-22

## Context

Phase 7 adds product capabilities without exposing protocol versions, processor
generations, parser versions or internal rollout flags to users. Existing
Page Views, Browser Context, Visitors, Sessions and Dimensions behavior is
implemented across services, and the Phase 7 Custom Events, Web Vitals,
Conversions, Funnels and country Geo capabilities have stable boundaries.

## Decision

1. The machine-readable manifest under `protocol/capabilities/` is the
   source of truth for capability IDs, dependencies and cross-layer boundaries.
2. A capability is a product boundary, not a package, service, database table
   or user configuration record.
3. The canonical manifest defines ten implemented capability contracts.
   User configuration is a separate site-scoped document that may enable or
   disable implemented capabilities; it cannot alter the canonical manifest.
   Capability-specific settings remain empty in the PR1 MVP. Conversion and
   Funnel definitions are managed separately.
4. Capability configuration, ingest security policy, browser consent and
   operator authorization remain separate boundaries.
5. `analytics_enabled` remains the Phase 8 transition switch until PR5. PR1
   does not change its database or runtime semantics. PR2 maps its value to
   the four existing Phase 6 derived capabilities; Page Views remain enabled.
6. Disabling a capability does not delete historical data. Independent event
   capabilities reject new corresponding events; Page View-derived
   capabilities strip their payload fields and stop new facts. Re-enabling
   does not automatically backfill. Rebuild requirements remain specific to
   each capability and are recorded in the manifest.
7. Capability state is scoped to `site_id`. Ingest policy is scoped to
   `site_id + environment`; environment is resolved by the Collector and is
   not added to Event Protocol or analytics facts by this decision.

## Consequences

- TypeScript and Rust expose read-only capability ID adapters for type safety;
  they do not read environment variables or feature flags.
- Future implementation PRs must update the capability manifest, fixtures,
  contract tests, and the owning SDK/Collector/Processor/API/Dashboard layer.
- Phase 8 configuration must validate dependencies against this contract and
  must not expose protocol or processing implementation details.
