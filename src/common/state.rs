use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::Pool;

use crate::common::middlewares::rate_limit::RateLimitStore;
use crate::config::Config;

/// État partagé injecté dans les handlers via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<AsyncPgConnection>,
    pub config: Arc<Config>,
    pub rate_limit: RateLimitStore,
    /// Passe à `true` à la réception du signal d'arrêt : `/health/ready` renvoie alors 503.
    pub shutting_down: Arc<AtomicBool>,
}
