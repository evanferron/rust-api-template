use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::common::state::AppState;

pub mod dto;
pub mod handler;

pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(handler::health_check))
}
