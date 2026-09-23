use db_migrator::run;
use sqlx::postgres::PgPoolOptions;

const EXPECTED_MIGRATIONS: &[(i64, &str, &str)] = &[
    (
        20260919000100,
        "create raw events",
        "7340977e6ba99d5fa0f00479e8e5d7158388d06aa17a398f564b52821ecde3213a7ceafbc7f146a2cb8ed4b3e855efd7",
    ),
    (
        20260919000200,
        "create page view aggregates",
        "6d8bd57deb40963f327f65cd6ed259970fefea3cbc7d5f4521bd3ec10e74a589f74856bd5dcf30a29aa17751a4a01960",
    ),
    (
        20260921000300,
        "create phase6 pr1 metadata",
        "f391b67cb9586b75c5b3d294a5b7504ab36ad42111621b4807c37d1bdc9ea3865e6a6b2667cdabba77da3f5a7fc559a4",
    ),
    (
        20260921000400,
        "add phase6 watermark constraints",
        "9b70e4b4ef3b5cce0b6a275d2200291fded012ced328b135686f0a961ddcf96085d0c321f0cf1521c9599419a970a2cc",
    ),
    (
        20260922000500,
        "create phase6 pr3 derived",
        "2acf23350f34be0a1d052b78abe131c9191acd90a232cf96abc34c61733383ae9f28830724c88082aa9a2d64e4e4ca2c",
    ),
    (
        20260922000600,
        "add phase6 pr3 query indexes",
        "e74ff18616314d0039d510b42d411054a3f9d14cd9712e8ddfbf212ebad963247231289818e200bf542ac60a2f669ebd",
    ),
    (
        20260922000700,
        "create phase6 pr4 dimensions",
        "49fdaabb77834423beb44f88c7229276d39ca5fa5d39e428bcca11ffdfdd6db09d0895eba50f1970e1110076ec02d78e",
    ),
    (
        20260922000800,
        "deprecate protocol v2 flag",
        "8368271f14da20308fe96d8460b8de942529f7cf2a283c9b4f348b548156b62c3f1920fd56972c6cd0df4c6a82882e37",
    ),
];

#[tokio::test]
#[ignore = "requires PostgreSQL; run pnpm test:integration"]
async fn migrations_are_idempotent() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");

    run(&database_url)
        .await
        .expect("first migration run should succeed");
    run(&database_url)
        .await
        .expect("second migration run should be idempotent");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await
        .expect("integration database should be reachable");
    let applied: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT version, description, encode(checksum, 'hex')
         FROM _sqlx_migrations
         ORDER BY version",
    )
    .fetch_all(&pool)
    .await
    .expect("SQLx migration history should be queryable");
    let expected = EXPECTED_MIGRATIONS
        .iter()
        .map(|(version, description, checksum)| {
            (*version, (*description).to_owned(), (*checksum).to_owned())
        })
        .collect::<Vec<_>>();
    assert_eq!(applied, expected);
}
