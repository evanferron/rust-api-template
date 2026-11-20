use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::common::state::AppState;

pub mod dto;
pub mod handler;
pub mod service;

/// Routes des posts (authentifiées via l'extracteur `AuthUser`).
/// Le chemin et la méthode de chaque route sont ceux déclarés dans `#[utoipa::path]`.
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handler::get_all, handler::create))
        .routes(routes!(
            handler::get_by_id,
            handler::update,
            handler::delete
        ))
}
