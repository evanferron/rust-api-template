pub mod swagger;

use crate::bootstrap::models::AppState;
use crate::modules::{auth, health, user};
use axum::Router;

/// Assemble tous les routers des modules en un seul router racine.
/// Branché sur `/api` dans server.rs via `.nest("/api", create_router(state))`.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes(state.clone()))
        .merge(user::routes(state))
}
