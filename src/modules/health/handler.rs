use axum::Json;

use crate::modules::health::dto::HealthResponse;

/// Vérifie que le serveur est opérationnel.
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Serveur opérationnel", body = HealthResponse),
    )
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
