use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ---------------------------------------------------------------------------
// Register
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    /// Adresse email unique de l'utilisateur
    #[schema(example = "evan@example.com")]
    pub email: String,

    /// Mot de passe en clair (haché côté service)
    #[schema(example = "S3cur3P@ssword")]
    pub password: String,

    /// Prénom affiché
    #[schema(example = "Evan")]
    pub first_name: String,

    /// Nom de famille
    #[schema(example = "Ferron")]
    pub last_name: String,
}

// ---------------------------------------------------------------------------
// Login
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    #[schema(example = "evan@example.com")]
    pub email: String,

    #[schema(example = "S3cur3P@ssword")]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    /// Durée de vie de l'access token en secondes
    pub expires_in: u32,
    pub token_type: String,
}

// ---------------------------------------------------------------------------
// Refresh
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub token_type: String,
}
