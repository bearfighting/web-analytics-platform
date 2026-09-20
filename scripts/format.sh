#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo fmt --all

pnpm exec prettier --write package.json pnpm-workspace.yaml tsconfig.base.json eslint.config.mjs compose.yaml compose.backend.yaml docker/README.md protocol/schemas protocol/examples protocol/fixtures protocol/http/fixtures packages scripts/validate-protocol.mjs scripts/validate-http-fixtures.mjs README.md docs/README.md docs/getting-started.md docs/event-protocol.md docs/phase-1-design.md docs/phase-2-design.md docs/phase-3-design.md docs/phase-4-design.md docs/roadmap.md docs/ingest-key.md docs/router-playground.md docs/decisions .github/workflows/ci.yml
exec pnpm --recursive --if-present format
