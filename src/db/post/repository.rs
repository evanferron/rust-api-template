use diesel::{QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use uuid::Uuid;

use crate::common::error::ApiError;
use crate::db::post::model::{NewPost, Post, PostChangeset};
use crate::db::schema::posts;

pub struct PostRepository;

// ---------------------------------------------------------------------------
// Capacités génériques (voir db::repository)
// ---------------------------------------------------------------------------

crate::impl_find_by_id!(PostRepository, Post, crate::db::schema::posts, Uuid);
crate::impl_delete!(PostRepository, crate::db::schema::posts, Uuid);
crate::impl_paginate!(PostRepository, Post, crate::db::schema::posts, created_at);

// ---------------------------------------------------------------------------
// Méthodes spécifiques à Post
// ---------------------------------------------------------------------------

impl PostRepository {
    /// Insère un nouveau post.
    /// La construction de `NewPost` est à la charge du service.
    pub async fn create(conn: &mut AsyncPgConnection, new_post: NewPost) -> Result<Post, ApiError> {
        diesel::insert_into(posts::table)
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
        diesel::update(posts::table.find(id))
            .set(&changeset)
            .returning(Post::as_returning())
            .get_result::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }
}
