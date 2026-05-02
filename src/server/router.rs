use crate::config::state::AppState;
use crate::modules::{auth, health, post, user};
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .nest("/health", health::routes())
        .nest("/auth", auth::routes(state.clone()))
        .nest("/users", user::routes(state.clone()))
        .nest("/posts", post::routes(state))
}
