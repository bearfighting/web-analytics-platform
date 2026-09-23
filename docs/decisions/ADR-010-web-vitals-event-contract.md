# ADR-010: Web Vitals event contract

Status: Accepted

PR4 uses the Event Protocol V1 tagged union for standard document-load Core Web Vitals and supporting FCP/TTFB metrics. A report explicitly references its Page View snapshot, is versioned by a monotonically increasing per-metric report sequence, and is aggregated independently from navigation analytics. The accepted measurement library is `web-vitals@6.2.2`; no custom PerformanceObserver calculations, soft-navigation metrics, attribution or configurable sampling are introduced.

The associated Page View UTC date is the reporting date. Collector validates metric bounds and frozen ratings; facts are idempotently upserted only by greater report sequence. Aggregates use discrete p75 only with at least four Page View samples.
