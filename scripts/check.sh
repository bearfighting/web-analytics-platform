#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

pnpm --filter @web-analytics/protocol-ts typecheck
pnpm --filter @web-analytics/observer-core typecheck
pnpm --filter @web-analytics/observer-next typecheck
pnpm --filter @web-analytics/analytics-core typecheck
pnpm --filter @web-analytics/analytics-browser typecheck
pnpm exec eslint packages --config eslint.config.mjs
pnpm --filter @web-analytics/nextjs-router-playground typecheck
pnpm --filter @web-analytics/nextjs-router-playground lint
pnpm format:check
