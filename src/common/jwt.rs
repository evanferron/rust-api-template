//! JWT : claims, création et vérification des tokens (access et refresh).
//!
//! Les deux types de tokens partagent `Claims` mais sont signés avec des secrets distincts
//! (`config.jwt.secret` / `config.jwt.refresh_secret`).

use axum::http::{HeaderMap, header::AUTHORIZATION};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::error::{ApiError, ErrorCode};

// ---------------------------------------------------------------------------
// Claims
// ---------------------------------------------------------------------------

/// Claims embarqués dans les JWT (access token et refresh token).
/// Les deux tokens partagent la même structure mais sont signés avec des secrets distincts.
/// Dans les handlers, utiliser l'extracteur `AuthUser` (`common::extract`) plutôt que les claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — UUID de l'utilisateur
    pub sub: Uuid,
    /// Issued at (timestamp Unix)
    pub iat: i64,
    /// Expiration (timestamp Unix)
    pub exp: i64,
}

// ---------------------------------------------------------------------------
// En-tête Authorization
// ---------------------------------------------------------------------------

/// Token de l'en-tête `Authorization: Bearer <token>`, s'il est présent et bien formé.
pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

// ---------------------------------------------------------------------------
// Clés HMAC
// ---------------------------------------------------------------------------

/// Règles de validation des tokens : HS256, expiration vérifiée sans tolérance.
fn validation() -> Validation {
    Validation {
        leeway: 0,
        ..Validation::default()
    }
}

/// Clé de signature HMAC. Échoue si le secret est vide.
fn encoding_key(secret: &str) -> Result<EncodingKey, ApiError> {
    EncodingKey::from_secret(secret.as_bytes()).map_err(ApiError::internal)
}

/// Clé de vérification HMAC. Échoue si le secret est vide.
fn decoding_key(secret: &str) -> Result<DecodingKey, ApiError> {
    DecodingKey::from_secret(secret.as_bytes()).map_err(ApiError::internal)
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
        &encoding_key(secret)?,
    )
    .map_err(ApiError::internal)
}

/// Vérifie et décode un access token JWT.
/// Erreurs : `TOKEN_EXPIRED`, `INVALID_TOKEN`.
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, ApiError> {
    decode_claims(token, secret)
}

/// Décode et vérifie un token (signature + expiration).
fn decode_claims(token: &str, secret: &str) -> Result<Claims, ApiError> {
    use jsonwebtoken::errors::ErrorKind;

    decode::<Claims>(token, &decoding_key(secret)?, &validation())
        .map(|data: TokenData<Claims>| data.claims)
        .map_err(|e| match e.kind() {
            ErrorKind::ExpiredSignature => ApiError::new(ErrorCode::TokenExpired),
            _ => ApiError::new(ErrorCode::InvalidToken).with_source(e),
        })
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
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::seconds(expiration as i64)).timestamp(),
    };

    encode(&Header::default(), &claims, &encoding_key(secret)?).map_err(ApiError::internal)
}

/// Vérifie et décode un refresh token JWT.
/// Erreurs : `TOKEN_EXPIRED`, `INVALID_TOKEN`.
pub fn verify_refresh_token(token: &str, secret: &str) -> Result<Claims, ApiError> {
    decode_claims(token, secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_token_parsing() {
        let mut headers = HeaderMap::new();
        assert_eq!(bearer_token(&headers), None);
        headers.insert(AUTHORIZATION, "Basic abc".parse().unwrap());
        assert_eq!(bearer_token(&headers), None);
        headers.insert(AUTHORIZATION, "Bearer ".parse().unwrap());
        assert_eq!(bearer_token(&headers), None);
        headers.insert(AUTHORIZATION, "Bearer abc.def".parse().unwrap());
        assert_eq!(bearer_token(&headers), Some("abc.def"));
    }

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
        use jsonwebtoken::{Header, encode};

        // Crée manuellement un token avec exp dans le passé
        let claims = Claims {
            sub: uuid::Uuid::new_v4(),
            iat: Utc::now().timestamp() - 3600,
            exp: Utc::now().timestamp() - 1800, // expiré il y a 30 minutes
        };

        let token = encode(
            &Header::default(),
            &claims,
            &encoding_key("secret").unwrap(),
        )
        .unwrap();

        let err = verify_token(&token, "secret").unwrap_err();
        assert_eq!(err.code(), ErrorCode::TokenExpired);
    }
}
