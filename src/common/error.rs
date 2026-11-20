//! # Gestion des erreurs
//!
//! Toutes les erreurs de l'API passent par [`ApiError`] et sont renvoyées au format
//! [RFC 9457](https://www.rfc-editor.org/rfc/rfc9457) (`application/problem+json`) :
//!
//! ```json
//! {
//!   "type": "about:blank",
//!   "title": "Not Found",
//!   "status": 404,
//!   "code": "POST_NOT_FOUND",
//!   "detail": "Post not found",
//!   "request_id": "4f1c…"
//! }
//! ```
//!
//! ## Ajouter un code d'erreur
//!
//! Une seule ligne dans `error_codes!` (plus bas) : `NomDuCode => (STATUT_HTTP, "Message par défaut"),`
//! Le code JSON (`NOM_DU_CODE`), le statut et la doc OpenAPI en découlent.
//!
//! ## Utilisation
//!
//! ```rust,ignore
//! // Code seul — le message par défaut est utilisé
//! return Err(ErrorCode::PostNotFound.into());
//! let post = repo.find(id).await?.ok_or(ErrorCode::PostNotFound)?;
//!
//! // Message spécifique pour le client
//! return Err(ApiError::new(ErrorCode::Forbidden).with_detail("You can only edit your own posts"));
//!
//! // Erreur technique : la cause est loggée, le client ne reçoit qu'un message générique
//! let hash = bcrypt::hash(pwd, cost).map_err(ApiError::internal)?;
//! ```
//!
//! Les erreurs Diesel, du pool et de `validator` se convertissent automatiquement avec `?`.

use std::collections::BTreeMap;
use std::fmt;

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::{ValidationErrors, ValidationErrorsKind};

use crate::common::middlewares::request_id;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// ---------------------------------------------------------------------------
// Codes d'erreur
// ---------------------------------------------------------------------------

/// Déclare les codes d'erreur : `Variante => (STATUT_HTTP, "Message par défaut"),`
macro_rules! error_codes {
    ($( $(#[$attr:meta])* $variant:ident => ($status:ident, $message:literal), )*) => {
        /// Code d'erreur stable renvoyé dans le champ `code` (ex: `POST_NOT_FOUND`).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum ErrorCode {
            $( $(#[$attr])* $variant, )*
        }

        impl ErrorCode {
            /// Tous les codes déclarés.
            pub const ALL: &'static [ErrorCode] = &[$(ErrorCode::$variant,)*];

            /// Statut HTTP associé au code.
            pub const fn status(self) -> StatusCode {
                match self {
                    $(ErrorCode::$variant => StatusCode::$status,)*
                }
            }

            /// Message par défaut (utilisé si aucun `detail` n'est fourni).
            pub const fn message(self) -> &'static str {
                match self {
                    $(ErrorCode::$variant => $message,)*
                }
            }
        }
    };
}

error_codes! {
    // --- Requête invalide ---------------------------------------------------
    BadRequest           => (BAD_REQUEST, "Bad request"),
    MalformedJson        => (BAD_REQUEST, "Malformed JSON body"),
    InvalidPathParam     => (BAD_REQUEST, "Invalid path parameter"),
    InvalidQuery         => (BAD_REQUEST, "Invalid query parameters"),
    ValidationFailed     => (UNPROCESSABLE_ENTITY, "Validation failed"),
    UnsupportedMediaType => (UNSUPPORTED_MEDIA_TYPE, "Expected `Content-Type: application/json`"),
    PayloadTooLarge      => (PAYLOAD_TOO_LARGE, "Request body is too large"),

    // --- Authentification / autorisation ----------------------------------
    Unauthorized         => (UNAUTHORIZED, "Authentication required"),
    InvalidToken         => (UNAUTHORIZED, "Invalid token"),
    TokenExpired         => (UNAUTHORIZED, "Token has expired"),
    InvalidCredentials   => (UNAUTHORIZED, "Invalid email or password"),
    Forbidden            => (FORBIDDEN, "Access denied"),

    // --- Ressources ---------------------------------------------------------
    NotFound             => (NOT_FOUND, "Resource not found"),
    RouteNotFound        => (NOT_FOUND, "Route not found"),
    MethodNotAllowed     => (METHOD_NOT_ALLOWED, "Method not allowed"),
    Conflict             => (CONFLICT, "Resource conflicts with an existing one"),

    // --- Limites / disponibilité -------------------------------------------
    RateLimited          => (TOO_MANY_REQUESTS, "Too many requests"),
    RequestTimeout       => (REQUEST_TIMEOUT, "Request timed out"),
    ServiceUnavailable   => (SERVICE_UNAVAILABLE, "Service temporarily unavailable"),
    Internal             => (INTERNAL_SERVER_ERROR, "Internal server error"),

    // --- Métier : users -----------------------------------------------------
    UserNotFound         => (NOT_FOUND, "User not found"),
    EmailAlreadyUsed     => (CONFLICT, "Email is already in use"),
    InvalidCurrentPassword => (FORBIDDEN, "Current password is incorrect"),

    // --- Métier : posts -----------------------------------------------------
    PostNotFound         => (NOT_FOUND, "Post not found"),
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Même représentation que dans le JSON (SCREAMING_SNAKE_CASE)
        match serde_json::to_value(self) {
            Ok(serde_json::Value::String(s)) => f.write_str(&s),
            _ => write!(f, "{self:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// ApiError
// ---------------------------------------------------------------------------

/// Erreurs de validation par champ : `{"email": ["Invalid email address"]}`.
pub type FieldErrors = BTreeMap<String, Vec<String>>;

/// Erreur renvoyée par les handlers, services et repositories.
#[derive(Debug)]
pub struct ApiError {
    code: ErrorCode,
    /// Message pour le client (sinon `code.message()`).
    detail: Option<String>,
    /// Détail par champ pour `VALIDATION_FAILED`.
    errors: Option<FieldErrors>,
    /// En-tête `Retry-After` (secondes), pour `RATE_LIMITED`.
    retry_after: Option<u64>,
    /// Cause technique — loggée, jamais envoyée au client.
    source: Option<BoxError>,
}

impl ApiError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            detail: None,
            errors: None,
            retry_after: None,
            source: None,
        }
    }

    /// Erreur interne (500) : `err` est loggé, le client reçoit un message générique.
    pub fn internal(err: impl Into<BoxError>) -> Self {
        Self::new(ErrorCode::Internal).with_source(err)
    }

    /// Remplace le message par défaut du code.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Ajoute des erreurs par champ.
    pub fn with_field_errors(mut self, errors: FieldErrors) -> Self {
        self.errors = Some(errors);
        self
    }

    /// Ajoute l'en-tête `Retry-After`.
    pub fn with_retry_after(mut self, seconds: u64) -> Self {
        self.retry_after = Some(seconds);
        self
    }

    /// Attache la cause technique (loggée côté serveur uniquement).
    pub fn with_source(mut self, err: impl Into<BoxError>) -> Self {
        self.source = Some(err.into());
        self
    }

    pub fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn status(&self) -> StatusCode {
        self.code.status()
    }

    /// Message envoyé au client.
    pub fn detail(&self) -> &str {
        self.detail.as_deref().unwrap_or(self.code.message())
    }

    pub fn field_errors(&self) -> Option<&FieldErrors> {
        self.errors.as_ref()
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.detail())
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

impl From<ErrorCode> for ApiError {
    fn from(code: ErrorCode) -> Self {
        Self::new(code)
    }
}

// ---------------------------------------------------------------------------
// Conversions automatiques
// ---------------------------------------------------------------------------

impl From<DieselError> for ApiError {
    fn from(err: DieselError) -> Self {
        match &err {
            DieselError::NotFound => ApiError::new(ErrorCode::NotFound),
            // Violations de contraintes : le message Postgres (nom de contrainte, valeurs)
            // n'est pas exposé — il reste dans les logs via `source`.
            DieselError::DatabaseError(
                DatabaseErrorKind::UniqueViolation | DatabaseErrorKind::ForeignKeyViolation,
                _,
            ) => ApiError::new(ErrorCode::Conflict).with_source(err),
            _ => ApiError::internal(err),
        }
    }
}

impl From<diesel_async::pooled_connection::bb8::RunError> for ApiError {
    fn from(err: diesel_async::pooled_connection::bb8::RunError) -> Self {
        use diesel_async::pooled_connection::bb8::RunError;
        match err {
            RunError::TimedOut => ApiError::new(ErrorCode::ServiceUnavailable)
                .with_source("timed out waiting for a database connection"),
            RunError::User(e) => ApiError::internal(e),
        }
    }
}

impl From<ValidationErrors> for ApiError {
    fn from(errors: ValidationErrors) -> Self {
        let mut fields = FieldErrors::new();
        collect_field_errors(&errors, "", &mut fields);
        ApiError::new(ErrorCode::ValidationFailed).with_field_errors(fields)
    }
}

/// Aplatit les erreurs `validator` (y compris imbriquées) en `{"champ.sous_champ": [...]}`.
/// Les erreurs de niveau struct (`#[validate(schema(...))]`) sont sous la clé `__all__`.
fn collect_field_errors(errors: &ValidationErrors, prefix: &str, out: &mut FieldErrors) {
    for (field, kind) in errors.errors() {
        let path = if prefix.is_empty() {
            field.to_string()
        } else {
            format!("{prefix}.{field}")
        };
        match kind {
            ValidationErrorsKind::Field(list) => {
                let messages = list.iter().map(|e| {
                    e.message
                        .as_deref()
                        .map(str::to_string)
                        .unwrap_or_else(|| e.code.to_string())
                });
                out.entry(path).or_default().extend(messages);
            }
            ValidationErrorsKind::Struct(nested) => collect_field_errors(nested, &path, out),
            ValidationErrorsKind::List(items) => {
                for (index, nested) in items {
                    collect_field_errors(nested, &format!("{path}[{index}]"), out);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Réponse HTTP (RFC 9457)
// ---------------------------------------------------------------------------

/// Corps des réponses d'erreur (`application/problem+json`, RFC 9457).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Toujours `about:blank` : le type d'erreur est porté par `code`.
    #[serde(rename = "type")]
    #[schema(example = "about:blank")]
    pub problem_type: String,
    /// Libellé du statut HTTP.
    #[schema(example = "Not Found")]
    pub title: String,
    #[schema(example = 404)]
    pub status: u16,
    /// Code stable, à utiliser côté client pour identifier l'erreur.
    pub code: ErrorCode,
    /// Message lisible.
    #[schema(example = "Post not found")]
    pub detail: String,
    /// Erreurs par champ (uniquement pour `VALIDATION_FAILED`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<HashMap<String, Vec<String>>>, example = json!({"email": ["Invalid email address"]}))]
    pub errors: Option<FieldErrors>,
    /// Identifiant de la requête (aussi dans l'en-tête `x-request-id` et les logs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl From<&ApiError> for ErrorResponse {
    fn from(err: &ApiError) -> Self {
        let status = err.status();
        Self {
            problem_type: "about:blank".to_string(),
            title: status.canonical_reason().unwrap_or("Error").to_string(),
            status: status.as_u16(),
            code: err.code,
            detail: err.detail().to_string(),
            errors: err.errors.clone(),
            request_id: request_id::current(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();

        if status.is_server_error() {
            tracing::error!(
                code = %self.code,
                detail = self.detail(),
                source = self.source.as_ref().map(|e| e.to_string()),
                "request failed"
            );
        } else {
            tracing::debug!(
                code = %self.code,
                detail = self.detail(),
                source = self.source.as_ref().map(|e| e.to_string()),
                "request rejected"
            );
        }

        let body = serde_json::to_vec(&ErrorResponse::from(&self))
            .expect("ErrorResponse is always serializable");

        let mut response = (status, body).into_response();
        let headers = response.headers_mut();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        if let Some(seconds) = self.retry_after {
            headers.insert(header::RETRY_AFTER, HeaderValue::from(seconds));
        }
        response
    }
}

// ---------------------------------------------------------------------------
// Erreurs produites hors des handlers (router, couches tower)
// ---------------------------------------------------------------------------

/// Fallback du router : route inconnue.
pub async fn route_not_found() -> ApiError {
    ErrorCode::RouteNotFound.into()
}

/// Fallback du router : route connue, méthode non supportée.
pub async fn method_not_allowed() -> ApiError {
    ErrorCode::MethodNotAllowed.into()
}

/// Erreurs des couches tower (`HandleErrorLayer`) : timeout ou erreur inattendue.
pub async fn handle_layer_error(err: axum::BoxError) -> ApiError {
    if err.is::<tower::timeout::error::Elapsed>() {
        ErrorCode::RequestTimeout.into()
    } else {
        ApiError::internal(err)
    }
}

/// `CatchPanicLayer` : une panique dans un handler devient une 500 JSON (au lieu d'une connexion coupée).
pub fn handle_panic(panic: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("unknown panic")
        .to_string();
    ApiError::internal(format!("panic: {message}")).into_response()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use diesel::result::DatabaseErrorInformation;
    use std::collections::HashSet;
    use validator::Validate;

    async fn body_of(err: ApiError) -> (StatusCode, Option<String>, serde_json::Value) {
        let response = err.into_response();
        let status = response.status();
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|v| v.to_str().unwrap().to_string());
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            content_type,
            serde_json::from_slice(&bytes).unwrap(),
        )
    }

    #[test]
    fn every_code_has_a_unique_name_and_an_error_status() {
        let mut names = HashSet::new();
        for code in ErrorCode::ALL {
            assert!(names.insert(code.to_string()), "duplicate name {code}");
            let status = code.status();
            assert!(status.is_client_error() || status.is_server_error());
            assert!(!code.message().is_empty());
        }
    }

    #[test]
    fn code_is_serialized_in_screaming_snake_case() {
        assert_eq!(ErrorCode::PostNotFound.to_string(), "POST_NOT_FOUND");
        assert_eq!(
            serde_json::to_value(ErrorCode::EmailAlreadyUsed).unwrap(),
            "EMAIL_ALREADY_USED"
        );
    }

    #[tokio::test]
    async fn response_follows_rfc_9457() {
        let (status, content_type, body) = body_of(ErrorCode::PostNotFound.into()).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(content_type.as_deref(), Some("application/problem+json"));
        assert_eq!(body["type"], "about:blank");
        assert_eq!(body["title"], "Not Found");
        assert_eq!(body["status"], 404);
        assert_eq!(body["code"], "POST_NOT_FOUND");
        assert_eq!(body["detail"], "Post not found");
        assert!(body.get("errors").is_none());
    }

    #[tokio::test]
    async fn detail_overrides_default_message() {
        let err = ApiError::new(ErrorCode::Forbidden).with_detail("Not your post");
        let (status, _, body) = body_of(err).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["detail"], "Not your post");
    }

    #[tokio::test]
    async fn internal_error_does_not_leak_its_source() {
        let err = ApiError::internal("connection refused on 10.0.0.12:5432");
        let (status, _, body) = body_of(err).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["detail"], "Internal server error");
        assert!(!body.to_string().contains("10.0.0.12"));
    }

    #[tokio::test]
    async fn rate_limited_sets_retry_after() {
        let response = ApiError::new(ErrorCode::RateLimited)
            .with_retry_after(12)
            .into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()[header::RETRY_AFTER], "12");
    }

    #[derive(Validate)]
    struct Payload {
        #[validate(email(message = "Invalid email address"))]
        email: String,
        #[validate(length(min = 8))]
        password: String,
    }

    #[tokio::test]
    async fn validation_errors_are_reported_per_field() {
        let errors = Payload {
            email: "nope".into(),
            password: "short".into(),
        }
        .validate()
        .unwrap_err();

        let (status, _, body) = body_of(ApiError::from(errors)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "VALIDATION_FAILED");
        assert_eq!(body["errors"]["email"][0], "Invalid email address");
        // Sans message explicite, le code de la règle est utilisé
        assert_eq!(body["errors"]["password"][0], "length");
    }

    #[test]
    fn diesel_not_found_maps_to_not_found() {
        assert_eq!(
            ApiError::from(DieselError::NotFound).code(),
            ErrorCode::NotFound
        );
    }

    #[test]
    fn diesel_unique_violation_maps_to_conflict_without_leaking() {
        let err = ApiError::from(DieselError::DatabaseError(
            DatabaseErrorKind::UniqueViolation,
            Box::new(TestDbError("duplicate key value violates users_email_key")),
        ));
        assert_eq!(err.code(), ErrorCode::Conflict);
        assert!(!err.detail().contains("users_email_key"));
    }

    #[test]
    fn other_diesel_errors_are_internal() {
        let err = ApiError::from(DieselError::DatabaseError(
            DatabaseErrorKind::CheckViolation,
            Box::new(TestDbError("check failed")),
        ));
        assert_eq!(err.code(), ErrorCode::Internal);
        assert!(std::error::Error::source(&err).is_some());
    }

    #[tokio::test]
    async fn panic_becomes_internal_error_without_leaking_message() {
        let response = handle_panic(Box::new("secret detail"));
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(body.contains("INTERNAL"));
        assert!(!body.contains("secret detail"));
    }

    #[tokio::test]
    async fn timeout_maps_to_request_timeout() {
        let err = handle_layer_error(Box::new(tower::timeout::error::Elapsed::new())).await;
        assert_eq!(err.code(), ErrorCode::RequestTimeout);
    }

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
