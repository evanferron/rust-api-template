use diesel_async::AsyncPgConnection;

use crate::app::config::Config;
use crate::core::errors::ApiError;
use crate::db::user::repository::UserRepository;
use crate::modules::auth::dto::{LoginRequest, LoginResponse, RefreshResponse, RegisterRequest};
use crate::modules::auth::helpers::{create_refresh_token, create_token, verify_refresh_token};
use crate::modules::user::dto::UserResponse;

pub struct AuthService;

impl AuthService {
    // -----------------------------------------------------------------------
    // Register
    // -----------------------------------------------------------------------

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

        let password_hash = hash_password(&payload.password).await?;
        let user = UserRepository::create(conn, payload, password_hash).await?;
        Ok(UserResponse::from(user))
    }

    // -----------------------------------------------------------------------
    // Login
    // -----------------------------------------------------------------------

    /// Retourne le LoginResponse (access token) ET le refresh token séparément.
    /// Le handler se charge de placer le refresh token dans un cookie HttpOnly.
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

        let response = LoginResponse {
            access_token,
            expires_in: config.jwt.expiration,
            token_type: "Bearer".to_string(),
        };

        // On retourne le refresh token séparément pour que le handler le place en cookie
        Ok((response, refresh_token))
    }

    // -----------------------------------------------------------------------
    // Refresh
    // -----------------------------------------------------------------------

    /// Vérifie le refresh token, émet un nouvel access token ET un nouveau refresh token.
    /// La rotation du refresh token à chaque appel permet de détecter les vols de token :
    /// si un token volé est utilisé après l'utilisateur légitime, le leur sera invalide.
    pub fn refresh(
        config: &Config,
        refresh_token: &str,
    ) -> Result<(RefreshResponse, String), ApiError> {
        let claims = verify_refresh_token(refresh_token, &config.jwt.refresh_secret)?;

        let new_access_token = create_token(claims.sub, &config.jwt.secret, config.jwt.expiration)?;

        // Rotation — on émet un nouveau refresh token
        let new_refresh_token = create_refresh_token(
            claims.sub,
            &config.jwt.refresh_secret,
            config.jwt.refresh_expiration,
        )?;

        let response = RefreshResponse {
            access_token: new_access_token,
            expires_in: config.jwt.expiration,
            token_type: "Bearer".to_string(),
        };

        Ok((response, new_refresh_token))
    }
}

// ---------------------------------------------------------------------------
// Helpers bcrypt
// ---------------------------------------------------------------------------

async fn hash_password(password: &str) -> Result<String, ApiError> {
    let password = password.to_string();
    let cost = if cfg!(debug_assertions) { 4 } else { 12 };
    tokio::task::spawn_blocking(move || bcrypt::hash(&password, cost))
        .await
        .map_err(|e| ApiError::InternalServer(e.to_string()))?
        .map_err(|e| ApiError::InternalServer(format!("Failed to hash password: {}", e)))
}

async fn verify_password(password: &str, hash: &str) -> Result<(), ApiError> {
    let password = password.to_string();
    let hash = hash.to_string();
    let valid = tokio::task::spawn_blocking(move || bcrypt::verify(&password, &hash))
        .await
        .map_err(|e| ApiError::InternalServer(e.to_string()))?
        .map_err(|e| ApiError::InternalServer(format!("Failed to verify password: {}", e)))?;

    if !valid {
        return Err(ApiError::Authentication(
            "Invalid email or password".to_string(),
        ));
    }
    Ok(())
}
