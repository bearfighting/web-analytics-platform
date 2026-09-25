use sqlx::PgPool;

use crate::auth::AdminTokens;

#[derive(Clone)]
pub struct AppState {
    pub(crate) pool: PgPool,
    pub(crate) definition_version: String,
    pub(crate) admin_tokens: Option<AdminTokens>,
}

pub fn state_with_definition_version(pool: PgPool, definition_version: String) -> AppState {
    AppState {
        pool,
        definition_version,
        admin_tokens: None,
    }
}

pub fn state_with_admin_tokens(mut state: AppState, admin_tokens: Option<AdminTokens>) -> AppState {
    state.admin_tokens = admin_tokens;
    state
}

pub fn state(pool: PgPool) -> AppState {
    AppState {
        pool,
        definition_version: "1".to_owned(),
        admin_tokens: None,
    }
}
