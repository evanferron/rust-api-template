use crate::app::models::AppState;
use crate::core::middlewares::auth::require_auth;
use axum::{
    Router,
    routing::{delete, get, put},
};

pub mod dto;
pub mod handler;
pub mod service;

/// Routes protégées par JWT
/// Le middleware `require_auth` valide le token et injecte le claim dans les extensions
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/users", get(handler::get_all))
        .route("/users/:id", get(handler::get_by_id))
        .route("/users/:id", put(handler::update))
        .route("/users/:id", delete(handler::delete))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ))
        .with_state(state)
}
