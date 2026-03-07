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
    let mut validation = Validation::default();
    validation.leeway = 0;
    let token_data: TokenData<Claims> = decode(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
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
    let mut validation = Validation::default();
    validation.leeway = 0;

    let token_data: TokenData<RefreshClaims> = decode(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
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

pub async fn hash_password(password: &str) -> Result<String, ApiError> {
    let password = password.to_string();
    let cost = if cfg!(debug_assertions) { 4 } else { 12 };
    tokio::task::spawn_blocking(move || bcrypt::hash(&password, cost))
        .await
        .map_err(|e| ApiError::InternalServer(e.to_string()))?
        .map_err(|e| ApiError::InternalServer(format!("Failed to hash password: {}", e)))
}

pub async fn verify_password(password: &str, hash: &str) -> Result<(), ApiError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_verify_token() {
        let user_id = uuid::Uuid::new_v4();
        let token = create_token(user_id, "secret", 3600).unwrap();
        let claims = verify_token(&token, "secret").unwrap();
        assert_eq!(claims.sub, user_id);
    }

    #[test]
    fn test_verify_token_wrong_secret() {
        let token = create_token(uuid::Uuid::new_v4(), "secret", 3600).unwrap();
        assert!(verify_token(&token, "wrong_secret").is_err());
    }

    #[test]
    fn test_verify_expired_token() {
        use chrono::Utc;
        use jsonwebtoken::{EncodingKey, Header, encode};

        // Crée manuellement un token avec exp dans le passé
        let claims = Claims {
            sub: uuid::Uuid::new_v4(),
            iat: Utc::now().timestamp() - 3600,
            exp: Utc::now().timestamp() - 1800, // expiré il y a 30 minutes
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_bytes()),
        )
        .unwrap();

        let err = verify_token(&token, "secret").unwrap_err();
        assert!(matches!(err, ApiError::Authentication(_)));
    }
}
