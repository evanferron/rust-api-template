use axum::extract::Query;
use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use axum::{
    Json,
    extract::{FromRequest, Request},
};
use uuid::Uuid;
use validator::Validate;

use crate::core::errors::ApiError;

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
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        value
            .validate()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        Ok(ValidatedJson(value))
    }
}

pub struct ValidatedQuery<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        value
            .validate()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        Ok(ValidatedQuery(value))
    }
}

pub fn parse_uuid(id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(id).map_err(|_| ApiError::BadRequest(format!("'{}' is not a valid UUID", id)))
}

pub struct ValidatedPath<T>(pub T);

impl<T, S> FromRequestParts<S> for ValidatedPath<T>
where
    T: serde::de::DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Invalid path parameter: {}", e)))?;

        value
            .validate()
            .map_err(|e| ApiError::BadRequest(e.to_string()))?;

        Ok(ValidatedPath(value))
    }
}
