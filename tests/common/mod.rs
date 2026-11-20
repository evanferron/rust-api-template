use api::app::build_app;
use api::common::middlewares::rate_limit::RateLimitStore;
use api::common::state::AppState;
use api::config::{Config, RateLimitConfig};
use axum::Router;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use tokio::sync::OnceCell;

use std::num::NonZeroU32;
use std::sync::Arc;

static TEST_APP: OnceCell<Router> = OnceCell::const_new();

pub async fn get_test_app() -> &'static Router {
    TEST_APP
        .get_or_init(|| async { build_test_app().await })
        .await
}

// ---------------------------------------------------------------------------
// DB helpers
// ---------------------------------------------------------------------------

pub fn test_db_url() -> String {
    dotenvy::dotenv().ok();
    std::env::var("DATABASE_TEST_URL")
        .expect("DATABASE_TEST_URL must be set to run integration tests")
}

pub async fn test_pool() -> Pool<AsyncPgConnection> {
    let db_url = test_db_url();
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&db_url);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .await
        .expect("Failed to create test pool")
}

/// Vide la base de test (revert + run de toutes les migrations).
pub async fn reset_db() {
    api::db::reset_database(&test_db_url())
        .await
        .expect("Failed to reset test DB");
}

pub fn test_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("invalid test configuration")
}

// ---------------------------------------------------------------------------
// App builder — monte le router complet pour les tests e2e
// ---------------------------------------------------------------------------

/// Construit l'AppState et le Router complet identiques à la prod.
/// Les steps Cucumber envoient leurs requêtes via `tower::ServiceExt::oneshot`.
pub async fn build_test_app() -> Router {
    build_app(test_state().await)
}

/// État de test : base de test, config `.env`, limites de débit très hautes.
pub async fn test_state() -> AppState {
    let config = test_config();
    let pool = test_pool().await;

    AppState {
        pool,
        config: Arc::new(config),
        // limites hautes pour ne pas bloquer les tests
        rate_limit: RateLimitStore::new(&RateLimitConfig {
            requests_per_minute: NonZeroU32::new(10_000).unwrap(),
            auth_requests_per_minute: NonZeroU32::new(10_000).unwrap(),
        }),
        shutting_down: Default::default(),
    }
}
