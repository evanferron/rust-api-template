use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::app::models::AppState;
use crate::core::errors::{ApiError, ErrorResponse};
use crate::modules::auth::helpers::Claims;
use crate::modules::user::dto::{UpdateUserRequest, UserResponse};
use crate::modules::user::service::UserService;

// ---------------------------------------------------------------------------
// GET /users
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/users",
    tag = "users",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Liste des utilisateurs", body = Vec<UserResponse>),
        (status = 401, description = "Non authentifié",        body = ErrorResponse),
    )
)]
pub async fn get_all(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
) -> Result<Json<Vec<UserResponse>>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let users = UserService::get_all(&mut conn).await?;
    Ok(Json(users))
}

// ---------------------------------------------------------------------------
// GET /users/:id
// ---------------------------------------------------------------------------

#[utoipa::path(
    get,
    path = "/api/users/{id}",
    tag = "users",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "UUID de l'utilisateur")
    ),
    responses(
        (status = 200, description = "Utilisateur trouvé",  body = UserResponse),
        (status = 401, description = "Non authentifié",     body = ErrorResponse),
        (status = 404, description = "Utilisateur introuvable", body = ErrorResponse),
    )
)]
pub async fn get_by_id(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let user = UserService::get_by_id(&mut conn, id).await?;
    Ok(Json(user))
}

// ---------------------------------------------------------------------------
// PUT /users/:id
// ---------------------------------------------------------------------------

#[utoipa::path(
    put,
    path = "/api/users/{id}",
    tag = "users",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "UUID de l'utilisateur")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Utilisateur mis à jour",  body = UserResponse),
        (status = 400, description = "Données invalides",        body = ErrorResponse),
        (status = 401, description = "Non authentifié",          body = ErrorResponse),
        (status = 403, description = "Action non autorisée",     body = ErrorResponse),
        (status = 404, description = "Utilisateur introuvable",  body = ErrorResponse),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, ApiError> {
    // Un utilisateur ne peut modifier que son propre profil
    if claims.sub != id {
        return Err(ApiError::Authorization(
            "You can only update your own profile".to_string(),
        ));
    }

    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let user = UserService::update(&mut conn, id, payload).await?;
    Ok(Json(user))
}

// ---------------------------------------------------------------------------
// DELETE /users/:id
// ---------------------------------------------------------------------------

#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    tag = "users",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "UUID de l'utilisateur")
    ),
    responses(
        (status = 204, description = "Utilisateur supprimé"),
        (status = 401, description = "Non authentifié",         body = ErrorResponse),
        (status = 403, description = "Action non autorisée",    body = ErrorResponse),
        (status = 404, description = "Utilisateur introuvable", body = ErrorResponse),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    // Un utilisateur ne peut supprimer que son propre compte
    if claims.sub != id {
        return Err(ApiError::Authorization(
            "You can only delete your own account".to_string(),
        ));
    }

    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    UserService::delete(&mut conn, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
