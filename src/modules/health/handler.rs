use axum::Json;
use axum::extract::State;

use crate::common::error::{ApiError, ErrorResponse};
use crate::common::state::AppState;
use crate::modules::health::dto::HealthResponse;
use crate::modules::health::service;

/// Liveness : le process répond. N'accède pas à la base.
///
/// À utiliser pour `livenessProbe` (Kubernetes) et le `HEALTHCHECK` Docker.
#[utoipa::path(
    get,
    path = "/health/live",
    tag = "health",
    responses(
        (status = 200, description = "Process opérationnel", body = HealthResponse),
    )
)]
pub async fn live() -> Json<HealthResponse> {
    Json(HealthResponse::new("ok"))
}

/// Readiness : le serveur peut traiter du trafic (base joignable, pas en cours d'arrêt).
///
/// À utiliser pour `readinessProbe` (Kubernetes) et les load balancers.
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "health",
    responses(
        (status = 200, description = "Prêt à recevoir du trafic", body = HealthResponse),
        (status = 503, description = "Base injoignable ou arrêt en cours", body = ErrorResponse),
    )
)]
pub async fn ready(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    service::check_ready(&state).await?;
    Ok(Json(HealthResponse::new("ready")))
}
