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
    let config = test_config();
    let pool = test_pool().await;

    let state = AppState {
        pool,
        config: Arc::new(config),
        // limites hautes pour ne pas bloquer les tests
        rate_limit: RateLimitStore::new(&RateLimitConfig {
            requests_per_minute: NonZeroU32::new(10_000).unwrap(),
            auth_requests_per_minute: NonZeroU32::new(10_000).unwrap(),
        }),
    };

    // Monte le même router que le serveur réel
    build_app(state)
}

// ---------------------------------------------------------------------------
// Seed helpers
// ---------------------------------------------------------------------------

/// Crée un utilisateur et retourne son access token JWT.
/// Pratique pour les tests de routes protégées.
pub async fn seed_user_and_login(app: &Router, email: &str, password: &str) -> String {
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use tower::ServiceExt;

    // Register
    let register_body = serde_json::json!({
        "email": email,
        "password": password,
        "first_name": "Test",
        "last_name": "User"
    });

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(register_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Login
    let login_body = serde_json::json!({
        "email": email,
        "password": password,
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    json["access_token"].as_str().unwrap().to_string()
}
