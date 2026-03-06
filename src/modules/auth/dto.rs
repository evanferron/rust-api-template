use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {
    /// Adresse email unique de l'utilisateur
    #[schema(example = "evan@example.com")]
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    /// Mot de passe en clair (haché côté service)
    #[schema(example = "S3cur3P@ssword")]
    #[validate(length(
        min = 8,
        max = 100,
        message = "Password must be between 8 and 100 characters"
    ))]
    pub password: String,

    /// Prénom affiché
    #[schema(example = "Evan")]
    #[validate(length(
        min = 1,
        max = 100,
        message = "First name must be between 1 and 100 characters"
    ))]
    pub first_name: String,

    /// Nom de famille
    #[schema(example = "Ferron")]
    #[validate(length(
        min = 1,
        max = 100,
        message = "Last name must be between 1 and 100 characters"
    ))]
    pub last_name: String,
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    #[schema(example = "evan@example.com")]
    #[validate(email(message = "Invalid email address"))]
    pub email: String,

    #[schema(example = "S3cur3P@ssword")]
    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    /// Durée de vie de l'access token en secondes
    pub expires_in: u32,
    pub token_type: String,
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub token_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn test_register_valid() {
        let req = RegisterRequest {
            email: "evan@example.com".to_string(),
            password: "securepass".to_string(),
            first_name: "Evan".to_string(),
            last_name: "Ferron".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_register_invalid_email() {
        let req = RegisterRequest {
            email: "not-an-email".to_string(),
            password: "securepass".to_string(),
            first_name: "Evan".to_string(),
            last_name: "Ferron".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_register_password_too_short() {
        let req = RegisterRequest {
            email: "evan@example.com".to_string(),
            password: "short".to_string(),
            first_name: "Evan".to_string(),
            last_name: "Ferron".to_string(),
        };
        assert!(req.validate().is_err());
    }
}
