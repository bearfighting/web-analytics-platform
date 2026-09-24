# ADR-011: Geo Country Lookup and IP Privacy Boundary

- Status: Accepted
- Date: 2026-09-24

## Context

Phase 7 Geo PR1 needs country-level Page View reporting while keeping raw IP addresses out of analytics storage. Collector currently accepts browser events and persists the validated event payload. Geo lookup therefore has to use connection metadata transiently and attach only a low-precision result to the stored event.

## Decision

1. Geo PR1 supports country only, using an operator-supplied local MaxMind GeoLite2 Country MMDB. Collector does not download or call an external lookup service.
2. `GEOIP_DATABASE_PATH` is mandatory for the Collector backend profile. Startup fails if the database cannot be opened or is not a GeoLite2 Country database. The operator updates the read-only mounted file offline and restarts Collector.
3. Collector uses the TCP peer address by default. It reads `X-Forwarded-For` only when that peer matches an explicitly configured `GEOIP_TRUSTED_PROXIES` CIDR. It walks the chain from right to left, skipping trusted proxies, and uses the first non-trusted address. If the forwarding chain is missing, malformed, or contains only trusted proxies, the client address is unresolved and maps to `unknown`.
4. IP addresses are used only for in-memory lookup. They are not placed in event payloads, database columns, or application logs. Missing, private, unmapped, or malformed client addresses resolve to `unknown`.
5. A Page View's enrichment metadata stores ISO 3166-1 alpha-2 country code (or `unknown`), provider, MMDB database type/build release, and parser version. The Processor derives idempotent `geo_country_facts` from that metadata. Rebuilds reuse the saved country result and do not retain an IP for re-resolution.
6. The Analytics API groups Page Views by country code, includes `unknown`, and remains site/date-range scoped. Region/city is deferred until separate data-quality, privacy, and deployment evaluation.

## Consequences

- Offline deployments work when the operator mounts the licensed database file; no IP leaves the Collector.
- Updating the MMDB affects newly ingested Page Views. Historical country values are reproducible from stored enrichment metadata but cannot be re-resolved with a newer dataset because raw IPs are intentionally discarded.
- Unknown counts make lookup coverage observable in the country report. Geo PR2 requires an explicit retention and re-resolution decision.
