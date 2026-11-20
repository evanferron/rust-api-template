use diesel::ExpressionMethods;
use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::common::error::{ApiError, ErrorCode};
use crate::db::post::model::{NewPost, Post, PostChangeset};
use crate::db::post::repository::PostRepository;
use crate::db::repository::prelude::*;
use crate::db::schema::posts;
use crate::modules::post::dto::{CreatePostRequest, PostResponse, UpdatePostRequest};

/// Posts de l'utilisateur, paginés du plus récent au plus ancien.
pub async fn get_all_by_user(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    params: PaginationParams,
) -> Result<PaginatedResponse<PostResponse>, ApiError> {
    let page =
        PostRepository::find_paginated_by(conn, params, || Box::new(posts::user_id.eq(user_id)))
            .await?;
    Ok(page.map(PostResponse::from))
}

/// Post `id`. Un brouillon (`published = false`) n'est visible que par son auteur :
/// pour les autres, il n'existe pas (`POST_NOT_FOUND`).
pub async fn get_by_id(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    id: Uuid,
) -> Result<PostResponse, ApiError> {
    let post = find_existing(conn, id).await?;
    if !post.published && post.user_id != user_id {
        return Err(ErrorCode::PostNotFound.into());
    }
    Ok(PostResponse::from(post))
}

pub async fn create(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    payload: CreatePostRequest,
) -> Result<PostResponse, ApiError> {
    let new_post = NewPost {
        user_id,
        title: payload.title,
        content: payload.content,
        published: payload.published.unwrap_or(false),
    };
    Ok(PostResponse::from(
        PostRepository::create(conn, new_post).await?,
    ))
}

/// Modifie le post `id`. Seul son auteur (`user_id`) peut le faire.
pub async fn update(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    id: Uuid,
    payload: UpdatePostRequest,
) -> Result<PostResponse, ApiError> {
    let post = find_existing(conn, id).await?;
    ensure_owner(&post, user_id, "You can only update your own posts")?;

    let changeset = PostChangeset {
        title: payload.title,
        content: payload.content,
        published: payload.published,
    };
    Ok(PostResponse::from(
        PostRepository::update(conn, id, changeset).await?,
    ))
}

/// Supprime le post `id`. Seul son auteur (`user_id`) peut le faire.
pub async fn delete(conn: &mut AsyncPgConnection, user_id: Uuid, id: Uuid) -> Result<(), ApiError> {
    let post = find_existing(conn, id).await?;
    ensure_owner(&post, user_id, "You can only delete your own posts")?;

    PostRepository::delete(conn, id).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn find_existing(conn: &mut AsyncPgConnection, id: Uuid) -> Result<Post, ApiError> {
    PostRepository::find_by_id(conn, id)
        .await?
        .ok_or(ErrorCode::PostNotFound.into())
}

fn ensure_owner(post: &Post, user_id: Uuid, message: &str) -> Result<(), ApiError> {
    if post.user_id != user_id {
        return Err(ApiError::new(ErrorCode::Forbidden).with_detail(message));
    }
    Ok(())
}
