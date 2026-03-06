use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::errors::ApiError;

// ---------------------------------------------------------------------------
// Claims
// ---------------------------------------------------------------------------

/// Claims embarqués dans le JWT access token.
/// Injectés dans les extensions de la requête par `require_auth`,
/// puis extraits dans les handlers via `Extension<Claims>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — UUID de l'utilisateur
    pub sub: Uuid,
    /// Issued at (timestamp Unix)
    pub iat: i64,
    /// Expiration (timestamp Unix)
    pub exp: i64,
}

/// Claims embarqués dans le JWT refresh token.
/// Contient uniquement l'identité, pas de rôle ni de données sensibles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: Uuid,
    pub iat: i64,
    pub exp: i64,
}

// ---------------------------------------------------------------------------
// Access token
// ---------------------------------------------------------------------------

/// Crée un access token JWT signé pour l'utilisateur donné.
///
/// # Arguments
/// * `user_id`    — UUID de l'utilisateur
/// * `secret`     — clé secrète depuis `config.jwt.secret`
/// * `expiration` — durée de vie en secondes depuis `config.jwt.expiration`
pub fn create_token(user_id: Uuid, secret: &str, expiration: u32) -> Result<String, ApiError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(expiration as i64)).timestamp(),
    };

    encode(
        &Header::default(), // algorithme HS256 par défaut
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::InternalServer(format!("Failed to create token: {}", e)))
}

/// Vérifie et décode un access token JWT.
/// Retourne les `Claims` si le token est valide et non expiré.
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, ApiError> {
    let token_data: TokenData<Claims> = decode(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            ApiError::Authentication("Token has expired".to_string())
        }
        jsonwebtoken::errors::ErrorKind::InvalidToken
        | jsonwebtoken::errors::ErrorKind::InvalidSignature => {
            ApiError::Authentication("Invalid token".to_string())
        }
        _ => ApiError::Authentication(format!("Token validation failed: {}", e)),
    })?;

    Ok(token_data.claims)
}

// ---------------------------------------------------------------------------
// Refresh token
// ---------------------------------------------------------------------------

/// Crée un refresh token JWT signé.
/// Utilise un secret distinct de l'access token pour limiter la surface d'attaque.
pub fn create_refresh_token(
    user_id: Uuid,
    secret: &str,
    expiration: u32,
) -> Result<String, ApiError> {
    let now = Utc::now();
    let claims = RefreshClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(expiration as i64)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::InternalServer(format!("Failed to create refresh token: {}", e)))
}

/// Vérifie et décode un refresh token JWT.
pub fn verify_refresh_token(token: &str, secret: &str) -> Result<RefreshClaims, ApiError> {
    let token_data: TokenData<RefreshClaims> = decode(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| match e.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            ApiError::Authentication("Refresh token has expired".to_string())
        }
        jsonwebtoken::errors::ErrorKind::InvalidToken
        | jsonwebtoken::errors::ErrorKind::InvalidSignature => {
            ApiError::Authentication("Invalid refresh token".to_string())
        }
        _ => ApiError::Authentication(format!("Refresh token validation failed: {}", e)),
    })?;

    Ok(token_data.claims)
}
