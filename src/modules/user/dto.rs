use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

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

/// Conversion depuis le modèle Diesel vers le DTO de réponse.
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
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    #[schema(example = "evan@example.com")]
    pub email: Option<String>,

    #[schema(example = "Evan")]
    pub first_name: Option<String>,

    #[schema(example = "Ferron")]
    pub last_name: Option<String>,

    /// Si fourni, l'ancien mot de passe est requis pour confirmer le changement
    pub new_password: Option<String>,
    pub current_password: Option<String>,
}
