use crate::config::state::AppState;
use crate::core::middlewares::auth::require_auth;
use axum::{
    Router,
    routing::{delete, get, put},
};

pub mod dto;
pub mod handler;
pub mod service;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(handler::get_all).post(handler::create))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}", put(handler::update))
        .route("/{id}", delete(handler::delete))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ))
        .with_state(state)
}
