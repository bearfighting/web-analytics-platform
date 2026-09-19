use chrono::{DateTime, NaiveDate, Utc};
use processor::Processor;
use serde_json::json;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};

fn database_url() -> String {
    std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point to the integration PostgreSQL database")
}

async fn setup() -> (Processor, PgPool) {
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&database_url())
        .await
        .expect("integration database should be reachable");
    sqlx::query(
        "TRUNCATE raw_events, page_view_daily, page_view_routes, page_view_totals RESTART IDENTITY",
    )
    .execute(&pool)
    .await
    .expect("aggregate tables should be writable");
    let processor = Processor::connect(&database_url())
        .await
        .expect("processor should connect");
    (processor, pool)
}

async fn insert_raw_event(
    pool: &PgPool,
    id: &str,
    site_id: &str,
    occurred_at: DateTime<Utc>,
    path: &str,
) {
    sqlx::query(
        "INSERT INTO raw_events
            (site_id, event_id, schema_version, event_type, occurred_at,
             received_at, path, payload)
         VALUES ($1, $2, 1, 'page_view', $3, NOW(), $4, $5)",
    )
    .bind(site_id)
    .bind(id)
    .bind(occurred_at)
    .bind(path)
    .bind(json!({
        "schema_version": 1,
        "event_id": id,
        "type": "page_view",
        "site_id": site_id,
        "occurred_at": occurred_at.timestamp_millis(),
        "path": path
    }))
    .execute(pool)
    .await
    .expect("raw event should be insertable");
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn processes_daily_routes_and_totals_atomically() {
    let (processor, pool) = setup().await;
    let day = NaiveDate::from_ymd_opt(2026, 9, 18)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc();
    insert_raw_event(&pool, "01J00000000000000000000010", "site_a", day, "/").await;
    insert_raw_event(&pool, "01J00000000000000000000011", "site_a", day, "/about").await;
    insert_raw_event(&pool, "01J00000000000000000000012", "site_a", day, "/").await;

    assert_eq!(processor.process_all_once().await.unwrap(), 3);
    assert_eq!(processor.process_all_once().await.unwrap(), 0);

    let daily =
        sqlx::query("SELECT page_views FROM page_view_daily WHERE site_id = 'site_a' AND day = $1")
            .bind(day.date_naive())
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i64, _>("page_views");
    assert_eq!(daily, 3);

    let root = sqlx::query(
        "SELECT page_views FROM page_view_routes
         WHERE site_id = 'site_a' AND day = $1 AND path = '/'",
    )
    .bind(day.date_naive())
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<i64, _>("page_views");
    assert_eq!(root, 2);

    let total = sqlx::query("SELECT page_views FROM page_view_totals WHERE site_id = 'site_a'")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get::<i64, _>("page_views");
    assert_eq!(total, 3);

    let processed =
        sqlx::query("SELECT COUNT(*) AS count FROM raw_events WHERE processed_at IS NOT NULL")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i64, _>("count");
    assert_eq!(processed, 3);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn uses_occurred_at_utc_day_and_isolates_sites() {
    let (processor, pool) = setup().await;
    let late_event = DateTime::parse_from_rfc3339("2026-09-18T00:30:00-04:00")
        .unwrap()
        .with_timezone(&Utc);
    insert_raw_event(
        &pool,
        "01J00000000000000000000013",
        "site_a",
        late_event,
        "/late",
    )
    .await;
    insert_raw_event(
        &pool,
        "01J00000000000000000000014",
        "site_b",
        late_event,
        "/late",
    )
    .await;

    assert_eq!(processor.process_all_once().await.unwrap(), 2);
    let rows = sqlx::query("SELECT site_id, day, page_views FROM page_view_daily ORDER BY site_id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].get::<String, _>("site_id"), "site_a");
    assert_eq!(rows[1].get::<String, _>("site_id"), "site_b");
    assert_eq!(
        rows[0].get::<NaiveDate, _>("day"),
        NaiveDate::from_ymd_opt(2026, 9, 18).unwrap()
    );
    assert_eq!(rows[0].get::<i64, _>("page_views"), 1);
    assert_eq!(rows[1].get::<i64, _>("page_views"), 1);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn failed_aggregate_update_leaves_event_unprocessed() {
    let (processor, pool) = setup().await;
    let day = Utc::now();
    insert_raw_event(
        &pool,
        "01J00000000000000000000015",
        "site_a",
        day,
        "/failure",
    )
    .await;
    sqlx::query("DROP TABLE page_view_routes")
        .execute(&pool)
        .await
        .unwrap();

    assert!(processor.process_one().await.is_err());
    let processed = sqlx::query(
        "SELECT processed_at FROM raw_events WHERE event_id = '01J00000000000000000000015'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<Option<DateTime<Utc>>, _>("processed_at");
    assert!(processed.is_none());

    for table in ["page_view_daily", "page_view_totals"] {
        let count = sqlx::query(&format!("SELECT COUNT(*) AS count FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap()
            .get::<i64, _>("count");
        assert_eq!(count, 0, "{table} should be rolled back");
    }

    sqlx::query(
        "CREATE TABLE page_view_routes (
            site_id VARCHAR(64) NOT NULL,
            day DATE NOT NULL,
            path TEXT NOT NULL,
            page_views BIGINT NOT NULL DEFAULT 0,
            PRIMARY KEY (site_id, day, path)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn canonical_fixtures_match_processor_aggregates() {
    let fixture_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protocol/phase-3/fixtures");
    let mut fixture_paths = std::fs::read_dir(fixture_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    fixture_paths.sort();

    for fixture_path in fixture_paths {
        let (processor, pool) = setup().await;
        let fixture: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&fixture_path).unwrap()).unwrap();
        let received_at =
            DateTime::parse_from_rfc3339(fixture["input"]["received_at"].as_str().unwrap())
                .unwrap()
                .with_timezone(&Utc);

        for event in fixture["input"]["events"].as_array().unwrap() {
            insert_fixture_event(&pool, event, received_at).await;
        }

        let processed = processor.process_all_once().await.unwrap();
        let expected = &fixture["expected"];
        assert_eq!(
            processed,
            expected["raw_events"]["inserted"].as_u64().unwrap(),
            "{} processed count",
            fixture_path.display()
        );
        assert_aggregate_rows(&pool, expected, &fixture_path).await;
    }
}

async fn insert_fixture_event(
    pool: &PgPool,
    event: &serde_json::Value,
    received_at: DateTime<Utc>,
) {
    let occurred_at =
        DateTime::<Utc>::from_timestamp_millis(event["occurred_at"].as_i64().unwrap()).unwrap();
    sqlx::query(
        "INSERT INTO raw_events
            (site_id, event_id, schema_version, event_type, occurred_at,
             received_at, path, url, title, referrer, payload)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (site_id, event_id) DO NOTHING",
    )
    .bind(event["site_id"].as_str().unwrap())
    .bind(event["event_id"].as_str().unwrap())
    .bind(event["schema_version"].as_i64().unwrap() as i32)
    .bind(event["type"].as_str().unwrap())
    .bind(occurred_at)
    .bind(received_at)
    .bind(event["path"].as_str().unwrap())
    .bind(event.get("url").and_then(|value| value.as_str()))
    .bind(event.get("title").and_then(|value| value.as_str()))
    .bind(event.get("referrer").and_then(|value| value.as_str()))
    .bind(event)
    .execute(pool)
    .await
    .unwrap();
}

async fn assert_aggregate_rows(
    pool: &PgPool,
    expected: &serde_json::Value,
    fixture_path: &std::path::Path,
) {
    let daily = expected["page_view_daily"].as_array().unwrap();
    let routes = expected["page_view_routes"].as_array().unwrap();
    let totals = expected["page_view_totals"].as_array().unwrap();

    for (table, expected_count) in [
        ("page_view_daily", daily.len()),
        ("page_view_routes", routes.len()),
        ("page_view_totals", totals.len()),
    ] {
        let actual_count = sqlx::query(&format!("SELECT COUNT(*) AS count FROM {table}"))
            .fetch_one(pool)
            .await
            .unwrap()
            .get::<i64, _>("count") as usize;
        assert_eq!(actual_count, expected_count, "{table} in {fixture_path:?}");
    }

    for row in daily {
        let actual =
            sqlx::query("SELECT page_views FROM page_view_daily WHERE site_id = $1 AND day = $2")
                .bind(row["site_id"].as_str().unwrap())
                .bind(NaiveDate::parse_from_str(row["day"].as_str().unwrap(), "%Y-%m-%d").unwrap())
                .fetch_one(pool)
                .await
                .unwrap()
                .get::<i64, _>("page_views");
        assert_eq!(actual, row["page_views"].as_i64().unwrap());
    }

    for row in routes {
        let actual = sqlx::query(
            "SELECT page_views FROM page_view_routes
             WHERE site_id = $1 AND day = $2 AND path = $3",
        )
        .bind(row["site_id"].as_str().unwrap())
        .bind(NaiveDate::parse_from_str(row["day"].as_str().unwrap(), "%Y-%m-%d").unwrap())
        .bind(row["path"].as_str().unwrap())
        .fetch_one(pool)
        .await
        .unwrap()
        .get::<i64, _>("page_views");
        assert_eq!(actual, row["page_views"].as_i64().unwrap());
    }

    for row in totals {
        let actual = sqlx::query("SELECT page_views FROM page_view_totals WHERE site_id = $1")
            .bind(row["site_id"].as_str().unwrap())
            .fetch_one(pool)
            .await
            .unwrap()
            .get::<i64, _>("page_views");
        assert_eq!(actual, row["page_views"].as_i64().unwrap());
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn once_cli_processes_the_backlog() {
    let (_processor, pool) = setup().await;
    let day = Utc::now();
    insert_raw_event(&pool, "01J00000000000000000000016", "site_cli", day, "/cli").await;

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_processor"))
        .env("DATABASE_URL", database_url())
        .arg("--once")
        .status()
        .expect("processor binary should start");
    assert!(status.success());

    let processed = sqlx::query(
        "SELECT processed_at FROM raw_events WHERE event_id = '01J00000000000000000000016'",
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .get::<Option<DateTime<Utc>>, _>("processed_at");
    assert!(processed.is_some());
}
