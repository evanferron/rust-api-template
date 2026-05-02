use crate::config::state::AppState;
use crate::core::errors::{ApiError, ErrorResponse};
use crate::core::params::{PaginationQuery, UuidParam};
use crate::core::repository::PaginationParams;
use crate::core::validator::{ValidatedJson, ValidatedPath, ValidatedQuery};
use crate::modules::auth::helpers::Claims;
use crate::modules::post::dto::{CreatePostRequest, PostResponse, UpdatePostRequest};
use crate::modules::post::service;
use axum::{Extension, Json, extract::State, http::StatusCode};

#[utoipa::path(
    get, path = "/api/posts", tag = "posts",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<i64>, Query, description = "Numéro de page (défaut: 1)"),
        ("per_page" = Option<i64>, Query, description = "Éléments par page (défaut: 20, max: 100)"),
    ),
    responses(
        (status = 200, body = Vec<PostResponse>),
        (status = 401, body = ErrorResponse),
    )
)]
pub async fn get_all(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ValidatedQuery(pagination): ValidatedQuery<PaginationQuery>,
) -> Result<Json<Vec<PostResponse>>, ApiError> {
    let params = PaginationParams::new(
        pagination.page.unwrap_or(1),
        pagination.per_page.unwrap_or(20),
    );
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(
        service::get_all_by_user(&mut conn, claims.sub, params).await?,
    ))
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
    ValidatedPath(params): ValidatedPath<UuidParam>,
) -> Result<Json<PostResponse>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::get_by_id(&mut conn, params.id).await?))
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
    ValidatedPath(params): ValidatedPath<UuidParam>,
    ValidatedJson(payload): ValidatedJson<UpdatePostRequest>,
) -> Result<Json<PostResponse>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(
        service::update(&mut conn, params.id, claims.sub, payload).await?,
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
    ValidatedPath(params): ValidatedPath<UuidParam>,
) -> Result<StatusCode, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    service::delete(&mut conn, params.id, claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}
