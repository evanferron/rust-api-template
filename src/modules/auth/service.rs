use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::config::api_config::Config;
use crate::core::errors::ApiError;
use crate::db::user::model::NewUser;
use crate::db::user::repository::UserRepository;
use crate::modules::auth::dto::{LoginRequest, LoginResponse, RefreshResponse, RegisterRequest};
use crate::modules::auth::helpers::{create_refresh_token, create_token, verify_refresh_token};
use crate::modules::auth::helpers::{hash_password, verify_password};
use crate::modules::user::dto::UserResponse;
// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

pub async fn register(
    conn: &mut AsyncPgConnection,
    payload: RegisterRequest,
) -> Result<UserResponse, ApiError> {
    if UserRepository::find_by_email(conn, &payload.email)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflict(format!(
            "Email '{}' is already in use",
            payload.email
        )));
    }

    let hashed_password = hash_password(&payload.password).await?;
    let new_user = NewUser {
        id: Uuid::new_v4(),
        email: payload.email,
        password: hashed_password,
        first_name: payload.first_name,
        last_name: payload.last_name,
    };
    let user = UserRepository::create(conn, new_user).await?;
    Ok(UserResponse::from(user))
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

/// Retourne le LoginResponse ET le refresh token séparément.
/// Le handler place le refresh token dans un cookie HttpOnly.
pub async fn login(
    conn: &mut AsyncPgConnection,
    config: &Config,
    payload: LoginRequest,
) -> Result<(LoginResponse, String), ApiError> {
    let user = UserRepository::find_by_email(conn, &payload.email)
        .await?
        .ok_or_else(|| ApiError::Authentication("Invalid email or password".to_string()))?;

    verify_password(&payload.password, &user.password).await?;

    let access_token = create_token(user.id, &config.jwt.secret, config.jwt.expiration)?;
    let refresh_token = create_refresh_token(
        user.id,
        &config.jwt.refresh_secret,
        config.jwt.refresh_expiration,
    )?;

    Ok((
        LoginResponse {
            access_token,
            expires_in: config.jwt.expiration,
            token_type: "Bearer".to_string(),
        },
        refresh_token,
    ))
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

/// Vérifie le refresh token et retourne un nouvel access token + nouveau refresh token (rotation).
pub fn refresh(
    config: &Config,
    refresh_token: &str,
) -> Result<(RefreshResponse, String), ApiError> {
    let claims = verify_refresh_token(refresh_token, &config.jwt.refresh_secret)?;

    let new_access_token = create_token(claims.sub, &config.jwt.secret, config.jwt.expiration)?;
    let new_refresh_token = create_refresh_token(
        claims.sub,
        &config.jwt.refresh_secret,
        config.jwt.refresh_expiration,
    )?;

    Ok((
        RefreshResponse {
            access_token: new_access_token,
            expires_in: config.jwt.expiration,
            token_type: "Bearer".to_string(),
        },
        new_refresh_token,
    ))
}
