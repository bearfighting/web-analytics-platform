# Docker Development

The repository provides development containers for the Next.js Router Playground, Collector, Processor and Analytics API.

From the repository root:

```bash
pnpm docker:dev
```

The Playground is available at:

```text
http://localhost:3000
```

The Playground uses `MockTransport` by default. To send browser events to the Collector, set these values in `.env`:

```env
NEXT_PUBLIC_ANALYTICS_TRANSPORT=fetch
NEXT_PUBLIC_ANALYTICS_ENDPOINT=http://localhost:4001/v1/events
NEXT_PUBLIC_ANALYTICS_INGEST_KEY=public-key-example
NEXT_PUBLIC_ANALYTICS_SITE_ID=site_example
```

Start the Collector with the backend profile:

```bash
docker compose --profile backend up --build collector
```

To start the Playground only after the Collector is healthy, use the backend
Compose override:

```bash
docker compose \
  -f compose.yaml \
  -f compose.backend.yaml \
  --profile backend up --build
```

The same workflow is available as:

```bash
pnpm docker:backend
```

The default `pnpm docker:dev` workflow does not load this override. It keeps
the Playground on MockTransport and does not require the Collector.

The Collector is available at `http://localhost:4001` and exposes `GET /health`, `POST /v1/events`, and CORS preflight for `/v1/events`. POST requests require both an allowlisted `Origin` and the configured `X-Ingest-Key`; in the PostgreSQL-backed workflow accepted events are stored in `raw_events`. Each `site_id + Origin` is limited to 600 requests per minute.

Start the PostgreSQL-backed Collector, Processor and Analytics API workflow with:

```bash
pnpm docker:processing
```

The Analytics API is available at `http://localhost:4002`. It exposes `/health`, all-time site Overview, and date-range Reports endpoints. The API reads only the PostgreSQL aggregate tables.

Run the complete Analytics workflow with an isolated E2E Compose project:

```bash
pnpm e2e:analytics
```

The E2E harness uses ports `14001`, `14002` and `15432`, and does not remove the existing PostgreSQL volume.

Generate a key without modifying the TOML configuration:

```bash
cargo run -p collector -- key generate --site site_example --environment production
```

Add the output to the matching `ingest_keys` entry and configure the Website Origin in `allowed_origins` before sending local events. CORS preflight returns `204`; rate-limited requests return `429` with `Retry-After: 60`.

The Compose setup mounts the source directory and keeps dependency/build directories in named volumes. The Collector image caches Cargo registry dependencies during image build and keeps `/workspace/target` in a named volume for incremental compilation. The container builds `observer-next`, `analytics-browser`, and `transport` before starting the Playground. Changes to Playground source hot reload; after changing package source, restart the container so the package can be rebuilt. PostgreSQL, BeaconTransport, retries, and durable storage are intentionally not part of this integration PR.
