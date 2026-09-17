# `@web-analytics/analytics-browser`

Browser SDK runtime for turning `NavigationEvent` values into Protocol V1 page-view events.

This package supports injectable context, `beforeSend`, and immediate single-event batches through an injected `Transport`. It does not implement HTTP, buffering, retries, or a backend.
