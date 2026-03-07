use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::schema::users::dsl;
use crate::db::user::model::{NewUser, User, UserChangeset};

pub struct UserRepository;

crate::impl_base_repository!(UserRepository, User, crate::db::schema::users, Uuid);

impl UserRepository {
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

    /// Insère un nouvel utilisateur.
    /// La construction de `NewUser` est à la charge du service.
    pub async fn create(conn: &mut AsyncPgConnection, new_user: NewUser) -> Result<User, ApiError> {
        diesel::insert_into(dsl::users)
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
        diesel::update(dsl::users.find(id))
            .set(&changeset)
            .returning(User::as_returning())
            .get_result::<User>(conn)
            .await
            .map_err(ApiError::from)
    }
}
