#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

pnpm exec prettier --check package.json pnpm-workspace.yaml protocol scripts/validate-protocol.mjs docs/event-protocol.md docs/router-playground.md docs/decisions .github/workflows/ci.yml
exec pnpm --recursive --if-present format:check
