//! Extracteurs qui désérialisent **et** valident (`validator::Validate`) l'entrée.
//!
//! Chaque rejet est converti en [`ApiError`] (format RFC 9457) avec un code précis :
//! `MALFORMED_JSON`, `UNSUPPORTED_MEDIA_TYPE`, `PAYLOAD_TOO_LARGE`, `VALIDATION_FAILED`,
//! `INVALID_QUERY`, `INVALID_PATH_PARAM`.

use axum::Json;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Path, Query, Request};
use axum::http::StatusCode;
use axum::http::request::Parts;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::common::error::{ApiError, ErrorCode};

/// Paramètre de chemin `{id}` de type UUID : `ValidatedPath(params): ValidatedPath<UuidParam>`.
#[derive(Deserialize, Validate)]
pub struct UuidParam {
    pub id: Uuid,
}

/// Corps JSON validé.
pub struct ValidatedJson<T>(pub T);

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(json_rejection)?;
        value.validate()?;
        Ok(ValidatedJson(value))
    }
}

/// Paramètres de query validés.
pub struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) =
            Query::<T>::from_request_parts(parts, state)
                .await
                .map_err(|e: QueryRejection| {
                    ApiError::new(ErrorCode::InvalidQuery).with_detail(e.body_text())
                })?;
        value.validate()?;
        Ok(ValidatedQuery(value))
    }
}

/// Paramètres de chemin validés.
pub struct ValidatedPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedPath<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) =
            Path::<T>::from_request_parts(parts, state)
                .await
                .map_err(|e: PathRejection| {
                    ApiError::new(ErrorCode::InvalidPathParam).with_detail(e.body_text())
                })?;
        value.validate()?;
        Ok(ValidatedPath(value))
    }
}

fn json_rejection(rejection: JsonRejection) -> ApiError {
    let code = match &rejection {
        // JSON valide mais ne correspondant pas au type (champ manquant, inconnu, mauvais type)
        JsonRejection::JsonDataError(_) => ErrorCode::ValidationFailed,
        JsonRejection::JsonSyntaxError(_) => ErrorCode::MalformedJson,
        JsonRejection::MissingJsonContentType(_) => ErrorCode::UnsupportedMediaType,
        other if other.status() == StatusCode::PAYLOAD_TOO_LARGE => ErrorCode::PayloadTooLarge,
        _ => ErrorCode::BadRequest,
    };
    ApiError::new(code).with_detail(rejection.body_text())
}
