use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::post::model::{NewPost, Post};
use crate::db::schema::posts::dsl;
use crate::db::user::model::User;
use crate::modules::post::dto::{CreatePostRequest, UpdatePostRequest};

// ---------------------------------------------------------------------------
// Macros génériques
// ---------------------------------------------------------------------------

crate::impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);
crate::impl_exists!(PostRepository, crate::db::schema::posts, Uuid);
crate::impl_count!(PostRepository, crate::db::schema::posts);
crate::impl_find_paginated!(PostRepository, Post, crate::db::schema::posts, created_at);

pub struct PostRepository;

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

    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        payload: UpdatePostRequest,
    ) -> Result<Post, ApiError> {
        diesel::update(dsl::posts.find(id))
            .set(&PostChangeset {
                title: payload.title,
                content: payload.content,
                published: payload.published,
            })
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

// ---------------------------------------------------------------------------
// Changeset interne
// ---------------------------------------------------------------------------

#[derive(diesel::AsChangeset)]
#[diesel(table_name = crate::db::schema::posts)]
struct PostChangeset {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}
