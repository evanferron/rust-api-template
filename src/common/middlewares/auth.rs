use axum::http::header::AUTHORIZATION;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::common::error::{ApiError, ErrorCode};
use crate::common::jwt::verify_token;
use crate::common::state::AppState;

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
            return Err(ApiError::new(ErrorCode::Unauthorized)
                .with_detail("Invalid Authorization header format, expected 'Bearer <token>'"));
        }
        None => {
            return Err(
                ApiError::new(ErrorCode::Unauthorized).with_detail("Missing Authorization header")
            );
        }
    };

    // --- Validation du token ---
    // TOKEN_EXPIRED ou INVALID_TOKEN
    let claims = verify_token(token, &state.config.jwt.secret)?;

    // --- Injection des claims dans les extensions ---
    // Accessible dans les handlers via Extension<Claims>
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
