use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::core::errors::{ApiError, ErrorResponse};
use crate::core::validator::ValidatedJson;
use crate::modules::auth::helpers::Claims;
use crate::modules::user::dto::{UpdateUserRequest, UserResponse};
use crate::{infra::state::AppState, modules::user::service};

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
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
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
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::get_by_id(&mut conn, id).await?))
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
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    if claims.sub != id {
        return Err(ApiError::Authorization(
            "You can only update your own profile".to_string(),
        ));
    }
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::update(&mut conn, id, payload).await?))
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
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    if claims.sub != id {
        return Err(ApiError::Authorization(
            "You can only delete your own account".to_string(),
        ));
    }
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    service::delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
