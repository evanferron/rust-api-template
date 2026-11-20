use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::common::error::{ApiError, ErrorCode};
use crate::common::password::{hash_password, verify_password};
use crate::db::repository::prelude::*;
use crate::db::user::model::UserChangeset;
use crate::db::user::repository::UserRepository;
use crate::modules::user::dto::{UpdateUserRequest, UserResponse};

pub async fn get_all(conn: &mut AsyncPgConnection) -> Result<Vec<UserResponse>, ApiError> {
    let users = UserRepository::find_all(conn).await?;
    Ok(users.into_iter().map(UserResponse::from).collect())
}

pub async fn get_by_id(conn: &mut AsyncPgConnection, id: Uuid) -> Result<UserResponse, ApiError> {
    let user = UserRepository::find_by_id(conn, id)
        .await?
        .ok_or(ErrorCode::UserNotFound)?;
    Ok(UserResponse::from(user))
}

/// Modifie le profil `id`. Seul l'utilisateur lui-même (`actor_id`) peut le faire.
pub async fn update(
    conn: &mut AsyncPgConnection,
    actor_id: Uuid,
    id: Uuid,
    payload: UpdateUserRequest,
) -> Result<UserResponse, ApiError> {
    ensure_self(actor_id, id, "You can only update your own profile")?;

    let user = UserRepository::find_by_id(conn, id)
        .await?
        .ok_or(ErrorCode::UserNotFound)?;

    if let Some(email) = &payload.email
        && email != &user.email
        && UserRepository::find_by_email(conn, email).await?.is_some()
    {
        return Err(ErrorCode::EmailAlreadyUsed.into());
    }

    // La présence de current_password quand new_password est fourni
    // est garantie par la validation du DTO (validate_password_change).
    let new_password = match (&payload.new_password, &payload.current_password) {
        (Some(new_pwd), Some(current_pwd)) => {
            if !verify_password(current_pwd, &user.password).await? {
                return Err(ErrorCode::InvalidCurrentPassword.into());
            }
            Some(hash_password(new_pwd).await?)
        }
        _ => None,
    };

    let changeset = UserChangeset {
        email: payload.email,
        first_name: payload.first_name,
        last_name: payload.last_name,
        password: new_password,
    };
    UserRepository::update(conn, id, changeset)
        .await
        .map(UserResponse::from)
}

/// Supprime le compte `id`. Seul l'utilisateur lui-même (`actor_id`) peut le faire.
pub async fn delete(
    conn: &mut AsyncPgConnection,
    actor_id: Uuid,
    id: Uuid,
) -> Result<(), ApiError> {
    ensure_self(actor_id, id, "You can only delete your own account")?;

    match UserRepository::delete(conn, id).await? {
        true => Ok(()),
        false => Err(ErrorCode::UserNotFound.into()),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ensure_self(actor_id: Uuid, id: Uuid, message: &str) -> Result<(), ApiError> {
    if actor_id != id {
        return Err(ApiError::new(ErrorCode::Forbidden).with_detail(message));
    }
    Ok(())
}
