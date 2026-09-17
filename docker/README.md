# Docker Development

PR2 provides a development container for the Next.js Router Playground.

From the repository root:

```bash
pnpm docker:dev
```

The Playground is available at:

```text
http://localhost:3000
```

The Compose setup mounts the source directory and keeps dependency/build directories in named volumes. The container builds `observer-next` before starting the Playground. Changes to Playground source hot reload; after changing Adapter source, restart the container so the package can be rebuilt. PostgreSQL and backend services are intentionally not part of this PR.
