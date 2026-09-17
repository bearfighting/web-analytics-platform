#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

pnpm protocol:validate
pnpm --filter @web-analytics/protocol-ts test
pnpm --filter @web-analytics/observer-core test
pnpm --filter @web-analytics/observer-next test
pnpm --filter @web-analytics/analytics-core test
pnpm --filter @web-analytics/analytics-browser test
pnpm --filter @web-analytics/nextjs-router-playground test
