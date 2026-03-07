use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::db::post::model::Post;

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct PostResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl From<Post> for PostResponse {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            user_id: post.user_id,
            title: post.title,
            content: post.content,
            published: post.published,
            created_at: post.created_at,
            updated_at: post.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreatePostRequest {
    #[schema(example = "Mon premier post")]
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title must be between 1 and 255 characters"
    ))]
    pub title: String,

    #[schema(example = "Contenu du post...")]
    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: String,

    /// Si absent, le post est créé en brouillon (published = false)
    pub published: Option<bool>,
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdatePostRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title must be between 1 and 255 characters"
    ))]
    pub title: Option<String>,

    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: Option<String>,

    pub published: Option<bool>,
}
