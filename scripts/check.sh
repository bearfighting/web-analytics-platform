#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

pnpm --filter @web-analytics/nextjs-router-playground typecheck
pnpm --filter @web-analytics/nextjs-router-playground lint
pnpm format:check
