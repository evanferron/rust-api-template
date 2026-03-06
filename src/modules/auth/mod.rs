use crate::app::models::AppState;
use axum::{Router, routing::post};

pub mod dto;
pub mod handler;
pub mod helpers;
pub mod service;

/// Routes publiques d'authentification
pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/auth/register", post(handler::register))
        .route("/auth/login", post(handler::login))
        .route("/auth/refresh", post(handler::refresh))
        .with_state(state)
}
