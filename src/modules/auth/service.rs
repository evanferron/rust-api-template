use diesel_async::AsyncPgConnection;

use crate::app::config::Config;
use crate::core::errors::ApiError;
use crate::db::user::repository::UserRepository;
use crate::modules::auth::dto::{
    LoginRequest, LoginResponse, RefreshRequest, RefreshResponse, RegisterRequest,
};
use crate::modules::auth::helpers::{create_refresh_token, create_token, verify_refresh_token};
use crate::modules::user::dto::UserResponse;

pub struct AuthService;

impl AuthService {
    // -----------------------------------------------------------------------
    // Register
    // -----------------------------------------------------------------------

    pub async fn register(
        conn: &mut AsyncPgConnection,
        config: &Config,
        payload: RegisterRequest,
    ) -> Result<UserResponse, ApiError> {
        // Vérifie que l'email n'est pas déjà pris
        if UserRepository::find_by_email(conn, &payload.email)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(format!(
                "Email '{}' is already in use",
                payload.email
            )));
        }

        // Hachage du mot de passe
        let password_hash = hash_password(&payload.password)?;

        // Création en base
        let user = UserRepository::create(conn, payload, password_hash).await?;

        Ok(UserResponse::from(user))
    }

    // -----------------------------------------------------------------------
    // Login
    // -----------------------------------------------------------------------

    pub async fn login(
        conn: &mut AsyncPgConnection,
        config: &Config,
        payload: LoginRequest,
    ) -> Result<LoginResponse, ApiError> {
        // Récupère l'utilisateur par email
        let user = UserRepository::find_by_email(conn, &payload.email)
            .await?
            .ok_or_else(|| ApiError::Authentication("Invalid email or password".to_string()))?;

        // Vérifie le mot de passe — message volontairement générique pour éviter l'énumération
        verify_password(&payload.password, &user.password)?;

        // Génère les tokens
        let access_token = create_token(user.id, &config.jwt.secret, config.jwt.expiration)?;
        let refresh_token = create_refresh_token(
            user.id,
            &config.jwt.refresh_secret,
            config.jwt.refresh_expiration,
        )?;

        Ok(LoginResponse {
            access_token,
            refresh_token,
            expires_in: config.jwt.expiration,
            token_type: "Bearer".to_string(),
        })
    }

    // -----------------------------------------------------------------------
    // Refresh
    // -----------------------------------------------------------------------

    pub fn refresh(config: &Config, payload: RefreshRequest) -> Result<RefreshResponse, ApiError> {
        // Vérifie le refresh token avec son secret dédié
        let claims = verify_refresh_token(&payload.refresh_token, &config.jwt.refresh_secret)?;

        // Émet un nouvel access token
        let access_token = create_token(claims.sub, &config.jwt.secret, config.jwt.expiration)?;

        Ok(RefreshResponse {
            access_token,
            expires_in: config.jwt.expiration,
            token_type: "Bearer".to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// Helpers de hachage (bcrypt)
// ---------------------------------------------------------------------------

/// Hache un mot de passe avec bcrypt.
/// Le coût par défaut (12) est un bon compromis sécurité/performance.
fn hash_password(password: &str) -> Result<String, ApiError> {
    bcrypt::hash(password, 12)
        .map_err(|e| ApiError::InternalServer(format!("Failed to hash password: {}", e)))
}

/// Vérifie un mot de passe contre son hash bcrypt.
fn verify_password(password: &str, hash: &str) -> Result<(), ApiError> {
    let valid = bcrypt::verify(password, hash)
        .map_err(|e| ApiError::InternalServer(format!("Failed to verify password: {}", e)))?;

    if !valid {
        return Err(ApiError::Authentication(
            "Invalid email or password".to_string(),
        ));
    }

    Ok(())
}
