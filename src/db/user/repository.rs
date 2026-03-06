use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::schema::users::dsl;
use crate::db::user::model::{NewUser, User};
use crate::modules::auth::dto::RegisterRequest;
use crate::modules::user::dto::UpdateUserRequest;

pub struct UserRepository;

impl UserRepository {
    // -----------------------------------------------------------------------
    // Find all
    // -----------------------------------------------------------------------

    pub async fn find_all(conn: &mut AsyncPgConnection) -> Result<Vec<User>, ApiError> {
        dsl::users
            .order(dsl::created_at.desc())
            .load::<User>(conn)
            .await
            .map_err(ApiError::from)
    }

    // -----------------------------------------------------------------------
    // Find by id
    // -----------------------------------------------------------------------

    pub async fn find_by_id(
        conn: &mut AsyncPgConnection,
        id: Uuid,
    ) -> Result<Option<User>, ApiError> {
        dsl::users
            .find(id)
            .first::<User>(conn)
            .await
            .optional()
            .map_err(ApiError::from)
    }

    // -----------------------------------------------------------------------
    // Find by email
    // -----------------------------------------------------------------------

    pub async fn find_by_email(
        conn: &mut AsyncPgConnection,
        email: &str,
    ) -> Result<Option<User>, ApiError> {
        dsl::users
            .filter(dsl::email.eq(email))
            .first::<User>(conn)
            .await
            .optional()
            .map_err(ApiError::from)
    }

    // -----------------------------------------------------------------------
    // Create
    // -----------------------------------------------------------------------

    pub async fn create(
        conn: &mut AsyncPgConnection,
        payload: RegisterRequest,
        password_hash: String,
    ) -> Result<User, ApiError> {
        let new_user = NewUser {
            id: Uuid::new_v4(),
            email: payload.email,
            password: password_hash,
            first_name: payload.first_name,
            last_name: payload.last_name,
        };

        diesel::insert_into(dsl::users)
            .values(&new_user)
            .get_result::<User>(conn)
            .await
            .map_err(ApiError::from)
    }

    // -----------------------------------------------------------------------
    // Update
    // -----------------------------------------------------------------------

    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        payload: UpdateUserRequest,
        new_password_hash: Option<String>,
    ) -> Result<User, ApiError> {
        // Diesel impose de passer par un struct qui implémente AsChangeset
        let changeset = UserChangeset {
            email: payload.email,
            first_name: payload.first_name,
            last_name: payload.last_name,
            password: new_password_hash,
        };

        diesel::update(dsl::users.find(id))
            .set(&changeset)
            .get_result::<User>(conn)
            .await
            .map_err(ApiError::from)
    }

    // -----------------------------------------------------------------------
    // Delete
    // -----------------------------------------------------------------------

    pub async fn delete(conn: &mut AsyncPgConnection, id: Uuid) -> Result<bool, ApiError> {
        let rows = diesel::delete(dsl::users.find(id))
            .execute(conn)
            .await
            .map_err(ApiError::from)?;

        Ok(rows > 0)
    }
}

// ---------------------------------------------------------------------------
// Changeset interne pour l'update partiel
// ---------------------------------------------------------------------------

/// Tous les champs sont Option — Diesel ne met à jour que les Some(_).
#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::users)]
struct UserChangeset {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub password: Option<String>,
}
