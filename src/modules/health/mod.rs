use axum::{Router, routing::get};

use crate::common::state::AppState;

pub mod dto;
pub mod handler;

pub fn routes() -> Router<AppState> {
    Router::new().route("/health", get(handler::health_check))
}
