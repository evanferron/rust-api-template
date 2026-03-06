use crate::app::models::AppState;
use crate::core::errors::{ApiError, ErrorResponse};
use crate::core::extractor::ValidatedJson;
use crate::modules::auth::dto::{LoginRequest, LoginResponse, RefreshResponse, RegisterRequest};
use crate::modules::auth::service::AuthService;
use crate::modules::user::dto::UserResponse;
use axum::response::IntoResponse;
use axum::{Json, extract::State};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use time::Duration;

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
    ValidatedJson(payload): ValidatedJson<RegisterRequest>,
) -> Result<(axum::http::StatusCode, Json<UserResponse>), ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let user = AuthService::register(&mut conn, payload).await?;
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
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let (response, refresh_token) = AuthService::login(&mut conn, &state.config, payload).await?;

    let cookie = Cookie::build(("refresh_token", refresh_token))
        .http_only(true) // inaccessible par JS
        .secure(true) // HTTPS uniquement en prod
        .same_site(SameSite::Strict) // protection CSRF
        .path("/api/auth/refresh") // cookie envoyé uniquement sur cette route
        .max_age(Duration::seconds(
            state.config.jwt.refresh_expiration as i64,
        ))
        .build();

    let jar = CookieJar::new().add(cookie);
    Ok((jar, Json(response)))
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

/// Émet un nouvel access token à partir d'un refresh token valide.
/// Le refresh token est lu depuis le cookie HttpOnly `refresh_token`
/// et un nouveau est émis en rotation.
//  ```
//  const response = await fetch("http://localhost:8080/api/auth/refresh", {
//     method: "POST",
//     credentials: "include", // ← au niveau du front il n'est pas nécessaire d'ajouter le token à la main, le cookie est envoyé automatiquement par le navigateur
//  });
//  ```
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    tag = "auth",
    params(
        (
            "refresh_token" = String,
            Cookie,
            description = "Refresh token HttpOnly envoyé automatiquement par le navigateur"
        )
    ),
    responses(
        (status = 200, description = "Token renouvelé avec succès",  body = RefreshResponse),
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
    let (response, new_refresh_token) = AuthService::refresh(&state.config, &refresh_token)?;
    let cookie = Cookie::build(("refresh_token", new_refresh_token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::seconds(
            state.config.jwt.refresh_expiration as i64,
        ))
        .build();
    Ok((jar.add(cookie), Json(response)))
}
