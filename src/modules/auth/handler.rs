use axum::{Json, extract::State};

use crate::app::models::AppState;
use crate::core::errors::{ApiError, ErrorResponse};
use crate::modules::auth::dto::{
    LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, RegisterRequest,
};
use crate::modules::auth::service::AuthService;
use crate::modules::user::dto::UserResponse;

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

/// Crée un nouveau compte utilisateur.
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Utilisateur créé avec succès", body = UserResponse),
        (status = 400, description = "Données invalides",            body = ErrorResponse),
        (status = 409, description = "Email déjà utilisé",           body = ErrorResponse),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(axum::http::StatusCode, Json<UserResponse>), ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let user = AuthService::register(&mut conn, &state.config, payload).await?;
    Ok((axum::http::StatusCode::CREATED, Json(user)))
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

/// Authentifie un utilisateur et retourne les tokens JWT.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authentification réussie", body = LoginResponse),
        (status = 400, description = "Données invalides",        body = ErrorResponse),
        (status = 401, description = "Identifiants incorrects",  body = ErrorResponse),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let response = AuthService::login(&mut conn, &state.config, payload).await?;
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

/// Émet un nouvel access token à partir d'un refresh token valide.
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token renouvelé",            body = RefreshResponse),
        (status = 401, description = "Refresh token invalide",     body = ErrorResponse),
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let response = AuthService::refresh(&state.config, payload)?;
    Ok(Json(response))
}
