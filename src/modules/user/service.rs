use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::user::model::UserChangeset;
use crate::db::user::repository::UserRepository;
use crate::modules::user::dto::{UpdateUserRequest, UserResponse};

pub async fn get_all(conn: &mut AsyncPgConnection) -> Result<Vec<UserResponse>, ApiError> {
    let users = UserRepository::find_all(conn).await?;
    Ok(users.into_iter().map(UserResponse::from).collect())
}

pub async fn get_by_id(conn: &mut AsyncPgConnection, id: Uuid) -> Result<UserResponse, ApiError> {
    UserRepository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", id)))
        .map(UserResponse::from)
}

pub async fn update(
    conn: &mut AsyncPgConnection,
    id: Uuid,
    payload: UpdateUserRequest,
) -> Result<UserResponse, ApiError> {
    let user = UserRepository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", id)))?;

    let new_password = match (&payload.new_password, &payload.current_password) {
        (Some(new_pwd), Some(current_pwd)) => {
            let current_pwd = current_pwd.clone();
            let hash = user.password.clone();
            let valid = tokio::task::spawn_blocking(move || bcrypt::verify(&current_pwd, &hash))
                .await
                .map_err(|e| ApiError::InternalServer(e.to_string()))?
                .map_err(|e| ApiError::InternalServer(e.to_string()))?;
            if !valid {
                return Err(ApiError::Authentication(
                    "Current password is incorrect".to_string(),
                ));
            }
            let new_pwd = new_pwd.clone();
            let cost = if cfg!(debug_assertions) { 4 } else { 12 };
            Some(
                tokio::task::spawn_blocking(move || bcrypt::hash(&new_pwd, cost))
                    .await
                    .map_err(|e| ApiError::InternalServer(e.to_string()))?
                    .map_err(|e| ApiError::InternalServer(e.to_string()))?,
            )
        }
        (Some(_), None) => {
            return Err(ApiError::BadRequest(
                "Current password is required".to_string(),
            ));
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

pub async fn delete(conn: &mut AsyncPgConnection, id: Uuid) -> Result<(), ApiError> {
    match UserRepository::delete(conn, id).await? {
        true => Ok(()),
        false => Err(ApiError::NotFound(format!("User '{}' not found", id))),
    }
}
