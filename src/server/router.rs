use crate::config::state::AppState;
use crate::modules::{auth, health, post, user};
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes(state.clone()))
        .merge(user::routes(state.clone()))
        .merge(post::routes(state))
}
