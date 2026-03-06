use axum::{Router, routing::get};

pub mod dto;
pub mod handler;

pub fn routes() -> Router {
    Router::new().route("/health", get(handler::health_check))
}
