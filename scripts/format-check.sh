#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

pnpm exec prettier --check package.json pnpm-workspace.yaml tsconfig.base.json eslint.config.mjs compose.yaml docker/README.md protocol packages scripts/validate-protocol.mjs docs/getting-started.md docs/event-protocol.md docs/phase-1-design.md docs/router-playground.md docs/decisions .github/workflows/ci.yml
exec pnpm --recursive --if-present format:check
