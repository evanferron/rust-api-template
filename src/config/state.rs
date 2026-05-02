use std::sync::Arc;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::Pool;
use serde::Deserialize;

use crate::config::config::Config;
use crate::core::middlewares::rate_limit::RateLimitStore;

// ---------------------------------------------------------------------------
// Config structs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration: u32,
    pub refresh_secret: String,
    pub refresh_expiration: u32,
}

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<AsyncPgConnection>,
    pub config: Arc<Config>,
    pub rate_limit: RateLimitStore,
}
