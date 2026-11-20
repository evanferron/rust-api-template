use axum::{Router, routing::get};

use crate::common::state::AppState;

pub mod dto;
pub mod handler;
pub mod service;

/// Routes des utilisateurs (authentifiées via l'extracteur `AuthUser`).
pub fn routes() -> Router<AppState> {
    Router::new().route("/users", get(handler::get_all)).route(
        "/users/{id}",
        get(handler::get_by_id)
            .put(handler::update)
            .delete(handler::delete),
    )
}
