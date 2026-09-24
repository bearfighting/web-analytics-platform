# PostgreSQL migrations

This directory is the source of truth for the PostgreSQL schema used by the
platform. Migrations are infrastructure-owned and are independent of the
Collector, Processor, and Analytics API services.

Run them through the standalone migrator:

```bash
DATABASE_URL=postgres://analytics:analytics@localhost:5432/analytics pnpm db:migrate
```

Migration files use one ordered SQLx migration history. Once a migration has
been applied, do not edit its filename, version, or SQL contents. Add a new
version for every subsequent schema change. New migrations should be
additive unless a destructive change has been separately reviewed and
approved.

Business services consume this schema but do not create or upgrade it during
startup. Deploy the migration job before deploying service versions that
require the new schema.

- `20260924001200_add_geo_country_facts.sql` stores country-only enrichment metadata and processor facts; raw client IP is never persisted.
