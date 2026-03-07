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

/// ─── Tests unitaires pour ApiError et ErrorResponse ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::StatusCode;
    use diesel::result::{DatabaseErrorInformation, DatabaseErrorKind, Error as DieselError};

    // ─── Helper pour lire le body JSON d'une Response ───────────────────────

    async fn parse_error_response(response: Response) -> ErrorResponse {
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let mut parsed: ErrorResponse = serde_json::from_slice(&bytes).unwrap();
        parsed.status = status.as_u16();
        parsed
    }

    // ─── IntoResponse — status codes ────────────────────────────────────────

    #[test]
    fn test_not_found_status() {
        let err = ApiError::NotFound("user".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_conflict_status() {
        let err = ApiError::Conflict("email".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn test_authentication_status() {
        let err = ApiError::Authentication("invalid token".to_string());
        assert_eq!(err.into_response().status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_authorization_status() {
        let err = ApiError::Authorization("forbidden".to_string());
        assert_eq!(err.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_bad_request_status() {
        let err = ApiError::BadRequest("invalid field".to_string());
        assert_eq!(err.into_response().status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_internal_server_status() {
        let err = ApiError::InternalServer("crash".to_string());
        assert_eq!(
            err.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_database_status() {
        let err = ApiError::Database("connection lost".to_string());
        assert_eq!(
            err.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_rate_limit_exceeded_status() {
        let err = ApiError::RateLimitExceeded {
            client_id: "127.0.0.1".to_string(),
            max_requests: 100,
            window_duration: std::time::Duration::from_secs(60),
        };
        assert_eq!(err.into_response().status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_serialization_error_status() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err = ApiError::Serialization(json_err);
        assert_eq!(
            err.into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    // ─── IntoResponse — body JSON ────────────────────────────────────────────

    #[tokio::test]
    async fn test_not_found_body() {
        let err = ApiError::NotFound("Post not found".to_string());
        let body = parse_error_response(err.into_response()).await;
        assert_eq!(body.status, 404);
        assert_eq!(body.error, "NOT_FOUND");
        assert_eq!(body.message, "Post not found");
    }

    #[tokio::test]
    async fn test_rate_limit_body_contains_client_id() {
        let err = ApiError::RateLimitExceeded {
            client_id: "user-abc".to_string(),
            max_requests: 10,
            window_duration: std::time::Duration::from_secs(30),
        };
        let body = parse_error_response(err.into_response()).await;
        assert_eq!(body.status, 429);
        assert!(body.message.contains("user-abc"));
        assert!(body.message.contains("10"));
        assert!(body.message.contains("30s"));
    }

    // ─── Display (thiserror) ─────────────────────────────────────────────────

    #[test]
    fn test_display_authentication() {
        let err = ApiError::Authentication("token expired".to_string());
        assert_eq!(err.to_string(), "Authentication error: token expired");
    }

    #[test]
    fn test_display_not_found() {
        let err = ApiError::NotFound("user 42".to_string());
        assert_eq!(err.to_string(), "Resource not found: user 42");
    }

    // ─── From<DieselError> ───────────────────────────────────────────────────

    #[test]
    fn test_from_diesel_not_found() {
        let err = ApiError::from(DieselError::NotFound);
        assert!(matches!(err, ApiError::NotFound(_)));
    }

    #[test]
    fn test_from_diesel_unique_violation() {
        let err = ApiError::from(DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new(TestDbError("duplicate key")),
        ));
        assert!(matches!(err, ApiError::Conflict(_)));
    }

    #[test]
    fn test_from_diesel_foreign_key_violation() {
        let err = ApiError::from(DieselError::DatabaseError(
            DatabaseErrorKind::ForeignKeyViolation,
            Box::new(TestDbError("fk violation")),
        ));
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn test_from_diesel_not_null_violation() {
        let err = ApiError::from(DieselError::DatabaseError(
            DatabaseErrorKind::NotNullViolation,
            Box::new(TestDbError("null field")),
        ));
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn test_from_diesel_check_violation() {
        let err = ApiError::from(DieselError::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            Box::new(TestDbError("check failed")),
        ));
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn test_from_diesel_unknown_db_error() {
        let err = ApiError::from(DieselError::DatabaseError(
            DatabaseErrorKind::UnableToSendCommand,
            Box::new(TestDbError("unknown")),
        ));
        assert!(matches!(err, ApiError::InternalServer(_)));
    }

    #[test]
    fn test_from_diesel_query_builder_error() {
        let err = ApiError::from(DieselError::QueryBuilderError("bad query".into()));
        assert!(matches!(err, ApiError::InternalServer(_)));
    }

    // ─── ErrorResponse Display ───────────────────────────────────────────────

    #[test]
    fn test_error_response_display_is_valid_json() {
        let resp = ErrorResponse {
            status: 404,
            error: "NOT_FOUND".to_string(),
            message: "Post not found".to_string(),
        };
        let displayed = resp.to_string();
        let parsed: serde_json::Value = serde_json::from_str(&displayed).unwrap();
        assert_eq!(parsed["status"], 404);
        assert_eq!(parsed["error"], "NOT_FOUND");
    }

    // ─── Stub DieselError pour les tests ─────────────────────────────────────

    struct TestDbError(&'static str);

    impl DatabaseErrorInformation for TestDbError {
        fn message(&self) -> &str {
            self.0
        }
        fn details(&self) -> Option<&str> {
            None
        }
        fn hint(&self) -> Option<&str> {
            None
        }
        fn table_name(&self) -> Option<&str> {
            None
        }
        fn column_name(&self) -> Option<&str> {
            None
        }
        fn constraint_name(&self) -> Option<&str> {
            None
        }
        fn statement_position(&self) -> Option<i32> {
            None
        }
    }
}
