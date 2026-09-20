#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo build --workspace

pnpm --filter @web-analytics/protocol-ts --filter @web-analytics/observer-core --filter @web-analytics/observer-next --filter @web-analytics/analytics-core --filter @web-analytics/analytics-browser --filter @web-analytics/transport build
pnpm --filter @web-analytics/nextjs-router-playground build
pnpm --filter @web-analytics/dashboard build
