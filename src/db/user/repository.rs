use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::common::error::ApiError;
use crate::db::schema::users;
use crate::db::user::model::{NewUser, User, UserChangeset};

pub struct UserRepository;

// ---------------------------------------------------------------------------
// Capacités génériques (voir db::repository)
// ---------------------------------------------------------------------------

crate::impl_find_by_id!(UserRepository, User, crate::db::schema::users, Uuid);
crate::impl_delete!(UserRepository, crate::db::schema::users, Uuid);
crate::impl_exists!(UserRepository, crate::db::schema::users, Uuid);
crate::impl_paginate!(UserRepository, User, crate::db::schema::users, created_at);

// ---------------------------------------------------------------------------
// Méthodes spécifiques à User
// ---------------------------------------------------------------------------

impl UserRepository {
    pub async fn find_by_email(
        conn: &mut AsyncPgConnection,
        email: &str,
    ) -> Result<Option<User>, ApiError> {
        users::table
            .filter(users::email.eq(email))
            .first::<User>(conn)
            .await
            .optional()
            .map_err(ApiError::from)
    }

    /// Insère un nouvel utilisateur.
    /// La construction de `NewUser` est à la charge du service.
    pub async fn create(conn: &mut AsyncPgConnection, new_user: NewUser) -> Result<User, ApiError> {
        diesel::insert_into(users::table)
            .values(&new_user)
            .returning(User::as_returning())
            .get_result::<User>(conn)
            .await
            .map_err(ApiError::from)
    }

    /// Met à jour un utilisateur existant.
    /// La construction de `UserChangeset` est à la charge du service.
    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        changeset: UserChangeset,
    ) -> Result<User, ApiError> {
        diesel::update(users::table.find(id))
            .set(&changeset)
            .returning(User::as_returning())
            .get_result::<User>(conn)
            .await
            .map_err(ApiError::from)
    }
}
