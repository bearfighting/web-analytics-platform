# Configuration Contract

This directory freezes Phase 8 configuration semantics without implementing
persistence or runtime behavior.

- capabilities.schema.json and environment-policy.schema.json describe
  stored configuration documents.
- capability-update.schema.json and environment-policy-update.schema.json
  describe client mutation bodies.
- audit-event.schema.json permits only redacted change metadata.
- openapi.json freezes the protected admin API routes and wire behavior.
- fixtures/, api-mutation-cases.json and scripts/validate-configuration-contract.mjs verify schema,
  migration defaults, dependencies, scope, auth, version conflicts, key
  display and audit redaction.

The canonical capability contract remains in
protocol/capabilities/capabilities.json. User configuration cannot change
that manifest.
