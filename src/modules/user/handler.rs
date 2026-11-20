use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::common::error::{ApiError, ErrorCode, ErrorResponse};
use crate::common::extract::{UuidParam, ValidatedJson, ValidatedPath};
use crate::common::jwt::Claims;
use crate::common::state::AppState;
use crate::modules::user::dto::{UpdateUserRequest, UserResponse};
use crate::modules::user::service;

#[utoipa::path(
    get, path = "/api/users", tag = "users",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Vec<UserResponse>),
        (status = 401, body = ErrorResponse),
    )
)]
pub async fn get_all(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    let mut conn = state.pool.get().await?;
    Ok(Json(service::get_all(&mut conn).await?))
}

#[utoipa::path(
    get, path = "/api/users/{id}", tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID de l'utilisateur")),
    responses(
        (status = 200, body = UserResponse),
        (status = 401, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    )
)]
pub async fn get_by_id(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    ValidatedPath(params): ValidatedPath<UuidParam>,
) -> Result<Json<UserResponse>, ApiError> {
    let mut conn = state.pool.get().await?;
    Ok(Json(service::get_by_id(&mut conn, params.id).await?))
}

#[utoipa::path(
    put, path = "/api/users/{id}", tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID de l'utilisateur")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, body = UserResponse),
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
    ValidatedJson(payload): ValidatedJson<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    if claims.sub != params.id {
        return Err(
            ApiError::new(ErrorCode::Forbidden).with_detail("You can only update your own profile")
        );
    }
    let mut conn = state.pool.get().await?;
    Ok(Json(service::update(&mut conn, params.id, payload).await?))
}

#[utoipa::path(
    delete, path = "/api/users/{id}", tag = "users",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID de l'utilisateur")),
    responses(
        (status = 204, description = "Utilisateur supprimé"),
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
    if claims.sub != params.id {
        return Err(
            ApiError::new(ErrorCode::Forbidden).with_detail("You can only delete your own account")
        );
    }
    let mut conn = state.pool.get().await?;
    service::delete(&mut conn, params.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
