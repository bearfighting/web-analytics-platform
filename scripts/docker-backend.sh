#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required to run the Docker backend environment." >&2
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose v2 is required. Use the 'docker compose' command." >&2
  exit 1
fi

COMPOSE=(docker compose \
  -f compose.yaml \
  -f compose.backend.yaml)

"${COMPOSE[@]}" --profile backend --profile storage up -d --wait postgres
"${COMPOSE[@]}" --profile backend --profile storage run --rm collector-migrate
exec "${COMPOSE[@]}" --profile backend --profile storage up --build
