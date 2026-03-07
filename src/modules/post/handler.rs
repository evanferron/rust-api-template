use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::core::errors::{ApiError, ErrorResponse};
use crate::core::validator::ValidatedJson;
use crate::infra::state::AppState;
use crate::modules::auth::helpers::Claims;
use crate::modules::post::dto::{CreatePostRequest, PostResponse, UpdatePostRequest};
use crate::modules::post::service;

#[utoipa::path(
    get, path = "/api/posts", tag = "posts",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Posts de l'utilisateur connecté", body = Vec<PostResponse>),
        (status = 401, body = ErrorResponse),
    )
)]
pub async fn get_all(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<PostResponse>>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::get_all_by_user(&mut conn, claims.sub).await?))
}

#[utoipa::path(
    get, path = "/api/posts/{id}", tag = "posts",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID du post")),
    responses(
        (status = 200, body = PostResponse),
        (status = 401, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    )
)]
pub async fn get_by_id(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<PostResponse>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::get_by_id(&mut conn, id).await?))
}

#[utoipa::path(
    post, path = "/api/posts", tag = "posts",
    security(("bearer_auth" = [])),
    request_body = CreatePostRequest,
    responses(
        (status = 201, body = PostResponse),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ValidatedJson(payload): ValidatedJson<CreatePostRequest>,
) -> Result<(StatusCode, Json<PostResponse>), ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let post = service::create(&mut conn, claims.sub, payload).await?;
    Ok((StatusCode::CREATED, Json(post)))
}

#[utoipa::path(
    put, path = "/api/posts/{id}", tag = "posts",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID du post")),
    request_body = UpdatePostRequest,
    responses(
        (status = 200, body = PostResponse),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdatePostRequest>,
) -> Result<Json<PostResponse>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(
        service::update(&mut conn, id, claims.sub, payload).await?,
    ))
}

#[utoipa::path(
    delete, path = "/api/posts/{id}", tag = "posts",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID du post")),
    responses(
        (status = 204, description = "Post supprimé"),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    service::delete(&mut conn, id, claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}
