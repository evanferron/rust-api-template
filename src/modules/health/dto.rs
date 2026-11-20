use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    #[schema(example = "ok")]
    pub status: String,
    #[schema(example = "0.1.0")]
    pub version: String,
}

impl HealthResponse {
    pub fn new(status: &str) -> Self {
        Self {
            status: status.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
