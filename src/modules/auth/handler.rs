use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration;

use crate::common::error::{ApiError, ErrorCode, ErrorResponse};
use crate::common::extract::ValidatedJson;
use crate::common::middlewares::rate_limit::AuthRateLimit;
use crate::common::state::AppState;
use crate::modules::auth::dto::{LoginRequest, LoginResponse, RefreshResponse, RegisterRequest};
use crate::modules::auth::service;
use crate::modules::user::dto::UserResponse;

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Utilisateur créé",    body = UserResponse),
        (status = 422, description = "Données invalides (VALIDATION_FAILED)", body = ErrorResponse),
        (status = 409, description = "Email déjà utilisé",  body = ErrorResponse),
    )
)]
pub async fn register(
    _: AuthRateLimit,
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let mut conn = state.pool.get().await?;
    let user = service::register(&mut conn, payload).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Authentification réussie", body = LoginResponse),
        (status = 422, description = "Données invalides (VALIDATION_FAILED)", body = ErrorResponse),
        (status = 401, description = "Identifiants incorrects",  body = ErrorResponse),
    )
)]
pub async fn login(
    _: AuthRateLimit,
    State(state): State<AppState>,
    jar: CookieJar,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.pool.get().await?;
    let (response, refresh_token) = service::login(&mut conn, &state.config, payload).await?;

    let cookie = refresh_cookie(&state, refresh_token, state.config.jwt.refresh_expiration);

    Ok((jar.add(cookie), Json(response)))
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    params(
        ("refresh_token" = String, Cookie, description = "Refresh token HttpOnly")
    ),
    responses(
        (status = 200, description = "Token renouvelé",             body = RefreshResponse),
        (status = 401, description = "Refresh token absent ou invalide", body = ErrorResponse),
    )
)]
pub async fn refresh(
    _: AuthRateLimit,
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, ApiError> {
    let refresh_token = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            ApiError::new(ErrorCode::Unauthorized).with_detail("Missing refresh token")
        })?;

    let (response, new_refresh_token) = service::refresh(&state.config, &refresh_token)?;

    let cookie = refresh_cookie(
        &state,
        new_refresh_token,
        state.config.jwt.refresh_expiration,
    );

    Ok((jar.add(cookie), Json(response)))
}

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Déconnexion réussie — cookie supprimé"),
    )
)]
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let cookie = refresh_cookie(&state, String::new(), 0);
    (jar.add(cookie), StatusCode::NO_CONTENT)
}

/// Cookie HttpOnly du refresh token, limité à la route de refresh.
/// `Secure` en production uniquement (pas de HTTPS en local).
fn refresh_cookie(state: &AppState, value: String, max_age_secs: u32) -> Cookie<'static> {
    Cookie::build(("refresh_token", value))
        .http_only(true)
        .secure(state.config.server.environment.is_production())
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::seconds(i64::from(max_age_secs)))
        .build()
}
