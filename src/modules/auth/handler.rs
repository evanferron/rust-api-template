use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration;

use crate::config::state::AppState;
use crate::core::errors::{ApiError, ErrorResponse};
use crate::core::validator::ValidatedJson;
use crate::modules::auth::dto::{LoginRequest, LoginResponse, RefreshResponse, RegisterRequest};
use crate::modules::auth::service;
use crate::modules::user::dto::UserResponse;

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "Utilisateur créé",    body = UserResponse),
        (status = 400, description = "Données invalides",   body = ErrorResponse),
        (status = 409, description = "Email déjà utilisé",  body = ErrorResponse),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    ValidatedJson(payload): ValidatedJson<RegisterRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let user = service::register(&mut conn, payload).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

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
    jar: CookieJar,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let (response, refresh_token) = service::login(&mut conn, &state.config, payload).await?;

    let cookie = Cookie::build(("refresh_token", refresh_token))
        .http_only(true)
        .secure(!cfg!(debug_assertions)) // false en dev (pas de HTTPS), true en prod
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::seconds(
            state.config.jwt.refresh_expiration as i64,
        ))
        .build();

    Ok((jar.add(cookie), Json(response)))
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
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
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, ApiError> {
    let refresh_token = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or_else(|| ApiError::Authentication("Missing refresh token".to_string()))?;

    let (response, new_refresh_token) = service::refresh(&state.config, &refresh_token)?;

    let cookie = Cookie::build(("refresh_token", new_refresh_token))
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::seconds(
            state.config.jwt.refresh_expiration as i64,
        ))
        .build();

    Ok((jar.add(cookie), Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Déconnexion réussie — cookie supprimé"),
        (status = 401, description = "Non authentifié", body = ErrorResponse),
    )
)]
pub async fn logout(jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build(("refresh_token", ""))
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::seconds(0))
        .build();

    (jar.add(cookie), StatusCode::NO_CONTENT)
}
