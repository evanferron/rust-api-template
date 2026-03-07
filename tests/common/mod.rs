use app::bootstrap::config::Config;
use app::bootstrap::models::AppState;
use app::bootstrap::router::create_router;
use app::core::middlewares::rate_limit::RateLimitStore;
use axum::Router;
use diesel::Connection;
use diesel::pg::PgConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tokio::sync::OnceCell;

use std::sync::Mutex;

static DB_LOCK: Mutex<()> = Mutex::new(());
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

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

pub fn reset_db() {
    let _guard = DB_LOCK.lock().unwrap(); // bloque jusqu'à ce que la DB soit libre

    let db_url = test_db_url();
    let mut conn =
        PgConnection::establish(&db_url).expect("Failed to connect to test DB for reset");

    conn.revert_all_migrations(MIGRATIONS)
        .expect("Failed to revert migrations");

    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations on test DB");
}

pub fn test_config() -> Config {
    dotenvy::dotenv().ok();
    Config::from_env().expect("Failed to load test config")
}

// ---------------------------------------------------------------------------
// App builder — monte le router complet pour les tests e2e
// ---------------------------------------------------------------------------

/// Construit l'AppState et le Router complet identiques à la prod.
/// Utilisé avec `axum_test::TestClient` pour simuler de vraies requêtes HTTP.
pub async fn build_test_app() -> Router {
    let config = test_config();
    let pool = test_pool().await;

    let state = AppState {
        pool,
        config,
        rate_limit: RateLimitStore::new(1000, 1000), // limites hautes pour ne pas bloquer les tests
    };

    // Monte le même router que le serveur réel
    Router::new().nest("/api", create_router(state))
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
