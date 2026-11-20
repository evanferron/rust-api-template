use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::common::state::AppState;

pub mod dto;
pub mod handler;
pub mod service;

/// Routes publiques d'authentification.
/// register / login / refresh sont limités par IP via l'extracteur `AuthRateLimit`.
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handler::register))
        .routes(routes!(handler::login))
        .routes(routes!(handler::refresh))
        .routes(routes!(handler::logout))
}
