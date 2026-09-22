# ADR-008：Internal Capability Boundaries

- Status: Accepted
- Date: 2026-09-22

## Context

Phase 7 adds product capabilities without exposing protocol versions, processor
generations, parser versions or internal rollout flags to users. Existing
Page Views, Browser Context, Visitors, Sessions and Dimensions behavior is
implemented across several services, while future Custom Events, Web Vitals,
Conversions, Funnels and Geo still need stable boundaries.

## Decision

1. The machine-readable manifest under `protocol/capabilities/v1/` is the
   source of truth for capability IDs, dependencies and cross-layer boundaries.
2. A capability is a product boundary, not a package, service, database table
   or user configuration record.
3. PR1 freezes ten capability contracts but only marks the existing five as
   implemented. Planned capabilities do not receive runtime branches, tables,
   flags or Dashboard controls until their implementation PR.
4. Capability configuration, ingest security policy, browser consent and
   operator authorization remain separate boundaries.
5. `analytics_enabled` remains the Phase 8 transition switch. PR1 does not
   change its database or runtime semantics.
6. Disabling a capability does not delete historical data. Rebuild and
   backfill requirements are capability-specific and are recorded in the
   manifest.

## Consequences

- TypeScript and Rust expose read-only capability ID adapters for type safety;
  they do not read environment variables or feature flags.
- Future implementation PRs must update the capability manifest, fixtures,
  contract tests, and the owning SDK/Collector/Processor/API/Dashboard layer.
- Phase 8 configuration must validate dependencies against this contract and
  must not expose protocol or processing implementation details.
