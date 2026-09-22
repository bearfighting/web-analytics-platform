#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required for migration regression tests." >&2
  exit 1
fi

if ! command -v psql >/dev/null 2>&1; then
  echo "psql is required for migration regression tests." >&2
  exit 1
fi

clean_database="analytics_migration_clean_${PPID}_$$"
upgrade_database="analytics_migration_upgrade_${PPID}_$$"
database_url_for() {
  node -e '
  const databaseUrl = new URL(process.argv[1]);
  databaseUrl.pathname = `/${process.argv[2]}`;
  process.stdout.write(databaseUrl.toString());
' "$1" "$2"
}
clean_database_url="$(database_url_for "$DATABASE_URL" "$clean_database")"
upgrade_database_url="$(database_url_for "$DATABASE_URL" "$upgrade_database")"

cleanup_upgrade_database() {
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS \"$clean_database\" WITH (FORCE)" >/dev/null 2>&1 || true
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS \"$upgrade_database\" WITH (FORCE)" >/dev/null 2>&1 || true
}
trap cleanup_upgrade_database EXIT

echo "Running the migration runner against the current schema..."
pnpm db:migrate

echo "Running the migration runner a second time to verify idempotency..."
pnpm db:migrate

echo "Running the db-migrator migration history regression test..."
cargo test -p db-migrator --test migrations -- --ignored --test-threads=1

echo "Creating an isolated clean database for first-install regression..."
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"$clean_database\"" >/dev/null

echo "Applying all migrations to the clean database..."
DATABASE_URL="$clean_database_url" pnpm db:migrate
DATABASE_URL="$clean_database_url" pnpm db:migrate
DATABASE_URL="$clean_database_url" cargo test -p db-migrator --test migrations -- --ignored --test-threads=1

echo "Creating an isolated database for migration upgrade regression..."
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "CREATE DATABASE \"$upgrade_database\"" >/dev/null

echo "Applying the previous migration history to the isolated database..."
DATABASE_URL="$upgrade_database_url" \
  cargo run -p db-migrator -- --target-version 20260921000400

echo "Upgrading the isolated database to the current migration history..."
DATABASE_URL="$upgrade_database_url" pnpm db:migrate
DATABASE_URL="$upgrade_database_url" cargo test -p db-migrator --test migrations -- --ignored --test-threads=1

check_schema() {
  local database_url="$1"

  psql "$database_url" -v ON_ERROR_STOP=1 <<'SQL'
DO $$
DECLARE
  expected_migrations constant bigint[] := ARRAY[
    20260919000100,
    20260919000200,
    20260921000300,
    20260921000400,
    20260922000500,
    20260922000600,
    20260922000700
  ];
  actual_migrations bigint[];
BEGIN
  SELECT array_agg(version ORDER BY version)
    INTO actual_migrations
    FROM _sqlx_migrations;

  IF actual_migrations IS DISTINCT FROM expected_migrations THEN
    RAISE EXCEPTION 'unexpected migration history: %', actual_migrations;
  END IF;
END
$$;

DO $$
DECLARE
  required_tables constant text[] := ARRAY[
    'raw_events',
    'page_view_totals',
    'page_view_daily',
    'page_view_routes',
    'analytics_feature_flags',
    'analytics_generations',
    'analytics_watermarks',
    'normalized_event_context',
    'visitor_event_facts',
    'session_events',
    'sessions',
    'visitor_daily',
    'session_daily',
    'dimension_event_facts',
    'dimension_daily'
  ];
  missing_table text;
BEGIN
  SELECT required
    INTO missing_table
    FROM unnest(required_tables) AS required
    WHERE to_regclass(format('public.%s', required)) IS NULL
    LIMIT 1;

  IF missing_table IS NOT NULL THEN
    RAISE EXCEPTION 'required migration table is missing: %', missing_table;
  END IF;
END
$$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1
      FROM pg_constraint
     WHERE conname = 'raw_events_site_event_unique'
       AND conrelid = 'public.raw_events'::regclass
  ) THEN
    RAISE EXCEPTION 'raw event idempotency constraint is missing';
  END IF;

  IF to_regclass('public.analytics_generations_one_active_per_site_idx') IS NULL THEN
    RAISE EXCEPTION 'generation active-site uniqueness index is missing';
  END IF;
END
$$;
SQL
}

echo "Checking clean schema objects..."
check_schema "$clean_database_url"

echo "Checking upgraded schema objects..."
check_schema "$upgrade_database_url"

echo "Checking current schema objects..."
check_schema "$DATABASE_URL"

echo "Migration regression passed."
