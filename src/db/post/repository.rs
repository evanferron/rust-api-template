use diesel::prelude::*;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::post::model::{NewPost, Post};
use crate::db::schema::posts::dsl;
use crate::modules::post::dto::CreatePostRequest;
use crate::modules::post::dto::UpdatePostRequest;

// Génère find_all, find_by_id, delete
crate::impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);

pub struct PostRepository;

impl PostRepository {
    // -----------------------------------------------------------------------
    // Méthodes spécifiques à Post
    // -----------------------------------------------------------------------

    /// Récupère tous les posts d'un utilisateur donné.
    pub async fn find_by_user_id(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
    ) -> Result<Vec<Post>, ApiError> {
        dsl::posts
            .filter(dsl::user_id.eq(user_id))
            .order(dsl::created_at.desc())
            .load::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }

    /// Récupère tous les posts publiés d'un utilisateur.
    pub async fn find_published_by_user_id(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
    ) -> Result<Vec<Post>, ApiError> {
        dsl::posts
            .filter(dsl::user_id.eq(user_id))
            .filter(dsl::published.eq(true))
            .order(dsl::created_at.desc())
            .load::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }

    /// Crée un nouveau post.
    pub async fn create(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
        payload: CreatePostRequest,
    ) -> Result<Post, ApiError> {
        let new_post = NewPost {
            id: Uuid::new_v4(),
            user_id,
            title: payload.title,
            content: payload.content,
            published: payload.published.unwrap_or(false),
        };

        diesel::insert_into(dsl::posts)
            .values(&new_post)
            .returning(Post::as_returning())
            .get_result::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }

    /// Met à jour un post existant.
    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        payload: UpdatePostRequest,
    ) -> Result<Post, ApiError> {
        let changeset = PostChangeset {
            title: payload.title,
            content: payload.content,
            published: payload.published,
        };

        diesel::update(dsl::posts.find(id))
            .set(&changeset)
            .returning(Post::as_returning())
            .get_result::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }

    // -----------------------------------------------------------------------
    // Exemple de join — Post avec son auteur
    // -----------------------------------------------------------------------

    /// Récupère un post avec les informations de son auteur via JOIN.
    /// Illustre l'utilisation de `belongs_to` + `inner_join` de Diesel.
    pub async fn find_with_author(
        conn: &mut AsyncPgConnection,
        post_id: Uuid,
    ) -> Result<Option<(Post, crate::db::user::model::User)>, ApiError> {
        use crate::db::schema::users;

        dsl::posts
            .inner_join(users::table) // JOIN grâce à joinable!(posts -> users)
            .filter(dsl::id.eq(post_id))
            .select((Post::as_select(), crate::db::user::model::User::as_select()))
            .first::<(Post, crate::db::user::model::User)>(conn)
            .await
            .optional()
            .map_err(ApiError::from)
    }
}

// ---------------------------------------------------------------------------
// Changeset interne
// ---------------------------------------------------------------------------

#[derive(AsChangeset)]
#[diesel(table_name = crate::db::schema::posts)]
struct PostChangeset {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}
