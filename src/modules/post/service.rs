use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::post::model::{NewPost, PostChangeset};
use crate::db::post::repository::PostRepository;
use crate::modules::post::dto::{CreatePostRequest, PostResponse, UpdatePostRequest};

pub async fn get_all_by_user(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
) -> Result<Vec<PostResponse>, ApiError> {
    let posts = PostRepository::find_by_user_id(conn, user_id).await?;
    Ok(posts.into_iter().map(PostResponse::from).collect())
}

pub async fn get_by_id(conn: &mut AsyncPgConnection, id: Uuid) -> Result<PostResponse, ApiError> {
    PostRepository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Post '{}' not found", id)))
        .map(PostResponse::from)
}

pub async fn create(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    payload: CreatePostRequest,
) -> Result<PostResponse, ApiError> {
    let new_post = NewPost {
        id: Uuid::new_v4(),
        user_id,
        title: payload.title,
        content: payload.content,
        published: payload.published.unwrap_or(false),
    };
    Ok(PostResponse::from(
        PostRepository::create(conn, new_post).await?,
    ))
}

pub async fn update(
    conn: &mut AsyncPgConnection,
    id: Uuid,
    user_id: Uuid,
    payload: UpdatePostRequest,
) -> Result<PostResponse, ApiError> {
    // Vérifie que le post existe et appartient à l'utilisateur
    let post = PostRepository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Post '{}' not found", id)))?;

    if post.user_id != user_id {
        return Err(ApiError::Authorization(
            "You can only update your own posts".to_string(),
        ));
    }

    let changeset = PostChangeset {
        title: payload.title,
        content: payload.content,
        published: payload.published,
    };
    Ok(PostResponse::from(
        PostRepository::update(conn, id, changeset).await?,
    ))
}

pub async fn delete(conn: &mut AsyncPgConnection, id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
    // Vérifie que le post appartient à l'utilisateur avant de supprimer
    let post = PostRepository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Post '{}' not found", id)))?;

    if post.user_id != user_id {
        return Err(ApiError::Authorization(
            "You can only delete your own posts".to_string(),
        ));
    }

    PostRepository::delete(conn, id).await?;
    Ok(())
}
