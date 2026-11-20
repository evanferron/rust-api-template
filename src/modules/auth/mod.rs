use axum::{Router, routing::post};

use crate::common::state::AppState;

pub mod dto;
pub mod handler;
pub mod helpers;
pub mod service;

/// Routes publiques d'authentification.
/// register / login / refresh sont limités par IP via l'extracteur `AuthRateLimit`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(handler::register))
        .route("/auth/login", post(handler::login))
        .route("/auth/refresh", post(handler::refresh))
        .route("/auth/logout", post(handler::logout))
}
