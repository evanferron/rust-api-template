use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    // --- Erreurs HTTP métier ---
    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Validation error: {0}")]
    BadRequest(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Conflict error: {0}")]
    Conflict(String),

    // --- Erreurs techniques ---
    #[error("Internal server error: {0}")]
    InternalServer(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    // --- Erreurs transverses ---
    #[error(
        "Rate limit exceeded for client {client_id}: max {max_requests} requests in {window_duration:?}"
    )]
    RateLimitExceeded {
        client_id: String,
        max_requests: u32,
        window_duration: std::time::Duration,
    },
}

// --- Conversion depuis Diesel ---

impl From<DieselError> for ApiError {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::NotFound => ApiError::NotFound("Resource not found".to_string()),

            DieselError::DatabaseError(kind, info) => match kind {
                DatabaseErrorKind::UniqueViolation => {
                    ApiError::Conflict(format!("Already exists: {}", info.message()))
                }
                DatabaseErrorKind::ForeignKeyViolation => {
                    ApiError::BadRequest(format!("Foreign key violation: {}", info.message()))
                }
                DatabaseErrorKind::NotNullViolation => {
                    ApiError::BadRequest(format!("Missing required field: {}", info.message()))
                }
                DatabaseErrorKind::CheckViolation => {
                    ApiError::BadRequest(format!("Constraint violation: {}", info.message()))
                }
                _ => {
                    tracing::error!("Unhandled database error: {}", info.message());
                    ApiError::InternalServer("Database error".to_string())
                }
            },

            DieselError::QueryBuilderError(e) => {
                tracing::error!("Query builder error: {}", e);
                ApiError::InternalServer("Query error".to_string())
            }

            DieselError::DeserializationError(e) => {
                tracing::error!("Deserialization error from Diesel: {}", e);
                ApiError::InternalServer("Data mapping error".to_string())
            }

            _ => {
                tracing::error!("Unexpected Diesel error: {}", err);
                ApiError::InternalServer("Unexpected database error".to_string())
            }
        }
    }
}

// --- Conversion depuis bb8 (pool de connexions async) ---

impl From<diesel_async::pooled_connection::bb8::RunError> for ApiError {
    fn from(err: diesel_async::pooled_connection::bb8::RunError) -> Self {
        tracing::error!("Connection pool error: {}", err);
        ApiError::InternalServer("Database connection unavailable".to_string())
    }
}

// --- Réponse HTTP ---

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub status: u16,
    pub error: String,
    pub message: String,
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            ApiError::Authentication(msg) => {
                (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone())
            }
            ApiError::Authorization(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),

            ApiError::InternalServer(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
                msg.clone(),
            ),
            ApiError::Database(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "DATABASE_ERROR",
                msg.clone(),
            ),
            ApiError::Serialization(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERIALIZATION_ERROR",
                err.to_string(),
            ),

            ApiError::RateLimitExceeded {
                client_id,
                max_requests,
                window_duration,
            } => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMIT_EXCEEDED",
                format!(
                    "Rate limit exceeded for client {}: max {} requests in {}s",
                    client_id,
                    max_requests,
                    window_duration.as_secs()
                ),
            ),
        };

        let body = Json(ErrorResponse {
            status: status.as_u16(),
            error: error_type.to_string(),
            message,
        });

        (status, body).into_response()
    }
}
