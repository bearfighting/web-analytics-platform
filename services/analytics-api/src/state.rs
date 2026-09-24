use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) definition_version: String,
}

pub fn state_with_definition_version(pool: PgPool, definition_version: String) -> AppState {
    AppState {
        pool,
        definition_version,
    }
}

pub fn state(pool: PgPool) -> AppState {
    AppState {
        pool,
        definition_version: "1".to_owned(),
    }
}
