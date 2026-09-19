use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
}

pub fn state(pool: PgPool) -> AppState {
    AppState { pool }
}
