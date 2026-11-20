use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::common::error::{ApiError, ErrorCode};
use crate::common::jwt::{create_refresh_token, create_token, verify_refresh_token};
use crate::common::password::{hash_password, verify_password};
use crate::config::Config;
use crate::db::user::model::NewUser;
use crate::db::user::repository::UserRepository;
use crate::modules::auth::dto::{LoginRequest, RegisterRequest, RegisterResponse, TokenResponse};
// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

pub async fn register(
    conn: &mut AsyncPgConnection,
    payload: RegisterRequest,
) -> Result<RegisterResponse, ApiError> {
    if UserRepository::find_by_email(conn, &payload.email)
        .await?
        .is_some()
    {
        return Err(ErrorCode::EmailAlreadyUsed.into());
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
    Ok(RegisterResponse::from(user))
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

/// Retourne l'access token ET le refresh token séparément.
/// Le handler place le refresh token dans un cookie HttpOnly.
pub async fn login(
    conn: &mut AsyncPgConnection,
    config: &Config,
    payload: LoginRequest,
) -> Result<(TokenResponse, String), ApiError> {
    let user = UserRepository::find_by_email(conn, &payload.email)
        .await?
        .ok_or(ErrorCode::InvalidCredentials)?;

    if !verify_password(&payload.password, &user.password).await? {
        return Err(ErrorCode::InvalidCredentials.into());
    }

    issue_tokens(config, user.id)
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

/// Vérifie le refresh token et retourne un nouvel access token + un nouveau refresh token.
/// L'ancien refresh token n'est pas révoqué : il reste valide jusqu'à son expiration.
pub fn refresh(config: &Config, refresh_token: &str) -> Result<(TokenResponse, String), ApiError> {
    let claims = verify_refresh_token(refresh_token, &config.jwt.refresh_secret)?;
    issue_tokens(config, claims.sub)
}

/// Crée un access token (réponse) et un refresh token (cookie) pour l'utilisateur.
fn issue_tokens(config: &Config, user_id: Uuid) -> Result<(TokenResponse, String), ApiError> {
    let access_token = create_token(user_id, &config.jwt.secret, config.jwt.expiration)?;
    let refresh_token = create_refresh_token(
        user_id,
        &config.jwt.refresh_secret,
        config.jwt.refresh_expiration,
    )?;

    Ok((
        TokenResponse {
            access_token,
            expires_in: config.jwt.expiration,
            token_type: "Bearer".to_string(),
        },
        refresh_token,
    ))
}
