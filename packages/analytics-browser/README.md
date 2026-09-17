# `@web-analytics/analytics-browser`

Browser SDK runtime for turning `NavigationEvent` values into Protocol V1 page-view events.

This package supports injectable context, `beforeSend`, a bounded in-memory buffer, scheduled/manual flush, and an injected `Transport`. It does not implement HTTP, persistence, retries, or a backend.
