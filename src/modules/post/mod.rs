use crate::core::middlewares::auth::require_auth;
use crate::core::middlewares::rate_limit::rate_limit_by_user;
use crate::infra::state::AppState;
use axum::{
    Router,
    routing::{delete, get, put},
};

pub mod dto;
pub mod handler;
pub mod service;

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/posts", get(handler::get_all).post(handler::create))
        .route("/posts/{id}", get(handler::get_by_id))
        .route("/posts/{id}", put(handler::update))
        .route("/posts/{id}", delete(handler::delete))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit_by_user,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ))
        .with_state(state)
}
