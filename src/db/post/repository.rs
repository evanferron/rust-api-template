use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::core::repository::PaginationParams;
use crate::db::post::model::{NewPost, Post, PostChangeset};
use crate::db::schema::posts::dsl;
use crate::db::user::model::User;

// ---------------------------------------------------------------------------
// Macros génériques
// ---------------------------------------------------------------------------

pub struct PostRepository;

crate::impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);
crate::impl_exists!(PostRepository, crate::db::schema::posts, Uuid);
crate::impl_count!(PostRepository, crate::db::schema::posts);
crate::impl_find_paginated!(PostRepository, Post, crate::db::schema::posts, created_at);

// ---------------------------------------------------------------------------
// Méthodes spécifiques à Post
// ---------------------------------------------------------------------------

impl PostRepository {
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

    pub async fn find_paginated_by_user(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
        params: PaginationParams,
    ) -> Result<Vec<Post>, ApiError> {
        dsl::posts
            .filter(dsl::user_id.eq(user_id))
            .order(dsl::created_at.desc())
            .limit(params.per_page)
            .offset(params.offset())
            .load::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }

    /// Insère un nouveau post.
    /// La construction de `NewPost` est à la charge du service.
    pub async fn create(conn: &mut AsyncPgConnection, new_post: NewPost) -> Result<Post, ApiError> {
        diesel::insert_into(dsl::posts)
            .values(&new_post)
            .returning(Post::as_returning())
            .get_result::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }

    /// Met à jour un post existant.
    /// La construction de `PostChangeset` est à la charge du service.
    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        changeset: PostChangeset,
    ) -> Result<Post, ApiError> {
        diesel::update(dsl::posts.find(id))
            .set(&changeset)
            .returning(Post::as_returning())
            .get_result::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }

    pub async fn find_with_author(
        conn: &mut AsyncPgConnection,
        post_id: Uuid,
    ) -> Result<Option<(Post, User)>, ApiError> {
        use crate::db::schema::users;
        use diesel::OptionalExtension;

        dsl::posts
            .inner_join(users::table)
            .filter(dsl::id.eq(post_id))
            .select((Post::as_select(), User::as_select()))
            .first::<(Post, User)>(conn)
            .await
            .optional()
            .map_err(ApiError::from)
    }
}
