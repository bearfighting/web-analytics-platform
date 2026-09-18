# Docker Development

The repository provides development containers for the Next.js Router Playground and the Phase 2 Collector.

From the repository root:

```bash
pnpm docker:dev
```

The Playground is available at:

```text
http://localhost:3000
```

Start the Collector with the backend profile:

```bash
docker compose --profile backend up --build collector
```

The Collector is available at `http://localhost:4001` and exposes `GET /health` and `POST /v1/events`. Accepted events are stored only in the process-local InMemory Sink.

The Compose setup mounts the source directory and keeps dependency/build directories in named volumes. The container builds `observer-next` before starting the Playground. Changes to Playground source hot reload; after changing Adapter source, restart the container so the package can be rebuilt. PostgreSQL, security controls, and durable storage are intentionally not part of this ingestion PR.
