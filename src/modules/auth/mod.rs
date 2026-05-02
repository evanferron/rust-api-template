use crate::config::state::AppState;
use crate::core::middlewares::rate_limit::rate_limit_by_ip;
use axum::{Router, routing::post};

pub mod dto;
pub mod handler;
pub mod helpers;
pub mod service;

/// Routes publiques d'authentification
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/register", post(handler::register))
        .route("/login", post(handler::login))
        .route("/refresh", post(handler::refresh))
        .route("/logout", post(handler::logout))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit_by_ip,
        ))
        .with_state(state)
}
