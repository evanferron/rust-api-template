use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};

use crate::db::user::model::User;

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

/// Représentation publique d'un utilisateur — ne contient jamais le hash du mot de passe.
#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

/// Tous les champs sont optionnels — seuls les champs fournis sont mis à jour.
#[derive(Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
#[validate(schema(function = "validate_password_change"))]
pub struct UpdateUserRequest {
    #[schema(example = "evan@example.com")]
    #[validate(email(message = "Invalid email address"))]
    pub email: Option<String>,

    #[schema(example = "Evan")]
    #[validate(length(
        min = 1,
        max = 100,
        message = "First name must be between 1 and 100 characters"
    ))]
    pub first_name: Option<String>,

    #[schema(example = "Ferron")]
    #[validate(length(
        min = 1,
        max = 100,
        message = "Last name must be between 1 and 100 characters"
    ))]
    pub last_name: Option<String>,

    /// Nouveau mot de passe — requiert current_password
    #[validate(length(
        min = 8,
        max = 100,
        message = "Password must be between 8 and 100 characters"
    ))]
    pub new_password: Option<String>,

    /// Mot de passe actuel — requis si new_password est fourni
    #[validate(length(min = 1, message = "Current password is required"))]
    pub current_password: Option<String>,
}

/// Validation au niveau du struct — vérifie la cohérence entre new_password et current_password.
/// Appelée après la validation des champs individuels.
fn validate_password_change(req: &UpdateUserRequest) -> Result<(), ValidationError> {
    match (&req.new_password, &req.current_password) {
        // new_password fourni sans current_password → erreur
        (Some(_), None) => {
            let mut err = ValidationError::new("password_change");
            err.message = Some("current_password is required when setting a new password".into());
            Err(err)
        }
        // current_password fourni sans new_password → inutile mais on laisse passer
        _ => Ok(()),
    }
}
