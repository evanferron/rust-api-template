use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::common::state::AppState;

pub mod dto;
pub mod handler;
pub mod service;

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handler::live))
        .routes(routes!(handler::ready))
}
