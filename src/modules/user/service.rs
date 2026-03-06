use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::user::repository::UserRepository;
use crate::modules::user::dto::{UpdateUserRequest, UserResponse};

pub struct UserService;

impl UserService {
    // -----------------------------------------------------------------------
    // Get all
    // -----------------------------------------------------------------------

    pub async fn get_all(conn: &mut AsyncPgConnection) -> Result<Vec<UserResponse>, ApiError> {
        let users = UserRepository::find_all(conn).await?;
        Ok(users.into_iter().map(UserResponse::from).collect())
    }

    // -----------------------------------------------------------------------
    // Get by id
    // -----------------------------------------------------------------------

    pub async fn get_by_id(
        conn: &mut AsyncPgConnection,
        id: Uuid,
    ) -> Result<UserResponse, ApiError> {
        let user = UserRepository::find_by_id(conn, id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", id)))?;

        Ok(UserResponse::from(user))
    }

    // -----------------------------------------------------------------------
    // Update
    // -----------------------------------------------------------------------

    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        payload: UpdateUserRequest,
    ) -> Result<UserResponse, ApiError> {
        // Vérifie que l'utilisateur existe
        let user = UserRepository::find_by_id(conn, id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", id)))?;

        // Changement de mot de passe — requiert l'ancien
        let new_password_hash = match (&payload.new_password, &payload.current_password) {
            (Some(new_pwd), Some(current_pwd)) => {
                let valid = bcrypt::verify(current_pwd, &user.password)
                    .map_err(|e| ApiError::InternalServer(e.to_string()))?;

                if !valid {
                    return Err(ApiError::Authentication(
                        "Current password is incorrect".to_string(),
                    ));
                }

                let hash = bcrypt::hash(new_pwd, 12)
                    .map_err(|e| ApiError::InternalServer(e.to_string()))?;

                Some(hash)
            }
            (Some(_), None) => {
                return Err(ApiError::BadRequest(
                    "Current password is required to set a new password".to_string(),
                ));
            }
            _ => None,
        };

        let updated = UserRepository::update(conn, id, payload, new_password_hash).await?;
        Ok(UserResponse::from(updated))
    }

    // -----------------------------------------------------------------------
    // Delete
    // -----------------------------------------------------------------------

    pub async fn delete(conn: &mut AsyncPgConnection, id: Uuid) -> Result<(), ApiError> {
        let deleted = UserRepository::delete(conn, id).await?;

        if !deleted {
            return Err(ApiError::NotFound(format!("User '{}' not found", id)));
        }

        Ok(())
    }
}
