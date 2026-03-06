use axum::http::header::AUTHORIZATION;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::bootstrap::models::AppState;
use crate::core::errors::ApiError;
use crate::modules::auth::helpers::verify_token;

/// Middleware de protection des routes par JWT.
///
/// Extrait et valide le Bearer token depuis le header Authorization,
/// puis injecte les claims dans les extensions de la requête
/// pour qu'ils soient disponibles dans les handlers via `Extension<Claims>`.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // --- Extraction du header Authorization ---
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        Some(_) => {
            return Err(ApiError::Authentication(
                "Invalid Authorization header format, expected 'Bearer <token>'".to_string(),
            ));
        }
        None => {
            return Err(ApiError::Authentication(
                "Missing Authorization header".to_string(),
            ));
        }
    };

    // --- Validation du token ---
    let claims = verify_token(token, &state.config.jwt.secret)
        .map_err(|e| ApiError::Authentication(e.to_string()))?;

    // --- Injection des claims dans les extensions ---
    // Accessible dans les handlers via Extension<Claims>
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
