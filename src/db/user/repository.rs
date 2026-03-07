use diesel::AsChangeset;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::schema::users::dsl;
use crate::db::user::model::{NewUser, User};
use crate::modules::auth::dto::RegisterRequest;
use crate::modules::user::dto::UpdateUserRequest;

pub struct UserRepository;

// Génération à la compilation des méthodes du repo génériques
crate::impl_base_repository!(UserRepository, User, crate::db::schema::users, Uuid);

impl UserRepository {
    // -----------------------------------------------------------------------
    // Méthodes spécifiques à User
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

    pub async fn create(
        conn: &mut AsyncPgConnection,
        payload: RegisterRequest,
        password: String,
    ) -> Result<User, ApiError> {
        let new_user = NewUser {
            id: Uuid::new_v4(),
            email: payload.email,
            password,
            first_name: payload.first_name,
            last_name: payload.last_name,
        };

        diesel::insert_into(dsl::users)
            .values(&new_user)
            .returning(User::as_returning())
            .get_result::<User>(conn)
            .await
            .map_err(ApiError::from)
    }

    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        payload: UpdateUserRequest,
        new_password: Option<String>,
    ) -> Result<User, ApiError> {
        let changeset = UserChangeset {
            email: payload.email,
            first_name: payload.first_name,
            last_name: payload.last_name,
            password: new_password,
        };

        diesel::update(dsl::users.find(id))
            .set(&changeset)
            .returning(User::as_returning())
            .get_result::<User>(conn)
            .await
            .map_err(ApiError::from)
    }
}

// ---------------------------------------------------------------------------
// Changeset interne
// ---------------------------------------------------------------------------

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::users)]
struct UserChangeset {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub password: Option<String>,
}
