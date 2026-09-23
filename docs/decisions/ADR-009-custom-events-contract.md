# ADR-009: Custom Events use bounded properties and site-scoped idempotency

- Status: Accepted (Phase 7 PR3 design)
- Date: 2026-09-23

## Context

The current protocol, SDK and Collector support Page Views only. The MVP requires product-defined actions such as signups and purchases, but arbitrary event payloads create unbounded storage, privacy and query risks. Custom Events must also fit the existing `site_id + event_id` idempotency model and preserve current Page View semantics.

## Decision

1. Add `type: "custom_event"` to the current event union without changing existing event shapes or shared batch semantics. A new independently tagged event type may be added to the current `schema_version`; older Collectors reject it until upgraded. Breaking changes to an existing event shape require a new public protocol version. Do not create a parallel protocol namespace or event endpoint.
2. Require `event_name` and an object-valued `properties`; an empty object is valid. Names are case-sensitive and constrained to 1–64 ASCII characters with a letter first and letters, digits, `_`, `.`, or `-` thereafter.
3. Bound properties to 32 total keys, depth 4, 20 items per array, 256 UTF-8 bytes per string and 8 KiB compact JSON total. Permit JSON scalar/array/object values only and reject invalid bounds.
4. Reject exact direct-identifier/authentication key names after case-folding and removing `_`, `-`, and `.` separators (email, emailaddress, useremail, phone, phonenumber, name, firstname, lastname, fullname, address, homeaddress, streetaddress, ip, ipaddress, useragent, cookie, password, passwd, token, userid and useridentifier). This does not catch sensitive values under arbitrary keys. Do not silently truncate or redact.
5. Use `(site_id, event_id)` for idempotency. The existing batch response acknowledges retries, while storage uniqueness makes them no-ops; first stored payload wins and retries do not create new facts or counts. A batch contains one site only. Event time is client `occurred_at`; Collector-owned `received_at` is used for freshness/diagnostics.
6. Store the complete event in `raw_events.payload`; make Page View-only `path` nullable for other event types. Keep properties out of derived fact columns and Analytics API responses in PR3.
7. Derive one rebuildable fact per event and report counts by site, exact event name and UTC event date. Custom Events never contribute to Page View, Visitor, Session or Browser Dimension metrics.
8. Expose a read-only site-scoped Events report and basic Dashboard count table. Event configuration and property-level querying are outside PR3.

## Alternatives considered

### Accept arbitrary unbounded JSON properties

Rejected because it permits pathological payloads, high storage costs and accidental collection of sensitive data.

### Store/query every property as a first-class dimension

Rejected for PR3 because it creates high-cardinality query behavior, an unstable API and broader privacy exposure. Raw properties remain available in the protected raw event for later Conversion/Funnel processing, but this report does not return them.

### Use a separate endpoint or internal protocol namespace for each event kind

Rejected because the existing batch and Collector workflow provides a versioned event envelope. A separately tagged event type can be added without changing existing event shapes. Older Collectors reject unknown types, so deployment must upgrade Collector support before SDKs send them. Breaking changes to an existing event shape still require a new public protocol version.

### Reject all property keys containing words such as “name”

Rejected because it also blocks ordinary business attributes such as `product_name`. The contract rejects a bounded list of exact normalized sensitive key names; integrators remain responsible for the values they send.

### Drop invalid properties or truncate them

Rejected because silent mutation makes client-side facts differ from accepted reports and complicates retries. The whole event (and its batch) receives a stable validation error.

## Consequences

- TypeScript and Rust protocol representations become discriminated unions; Collector validation and batch fixtures must support mixed event types.
- The database migration must permit `path = NULL` for Custom Events while preserving required Page View validation and all historical rows.
- Property limits and denylisted keys are protocol contract and require canonical fixtures in all validation layers.
- SDK consumers get a simple `event()` method, while the server remains authoritative for validation.
- Custom Event facts and reports are rebuildable and site-scoped; property-level analytics and event configuration require later design.

## Compatibility boundary

Existing Page View payloads, deduplication, storage, API and Dashboard metrics remain unchanged. New collectors may accept the new event type under the current protocol schema version; older collectors reject it as an unsupported event type. Mixed batches therefore require the new client to target a collector that has PR3 deployed. No transparent downgrade is attempted.
