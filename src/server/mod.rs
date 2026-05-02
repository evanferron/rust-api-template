pub mod router;
mod swagger;

use crate::config::api_config::Config;
use crate::config::state::AppState;
use crate::core::logger;
use crate::core::middlewares::rate_limit::RateLimitStore;
use crate::server::router::create_router;
use axum::BoxError;
use axum::Router;
use axum::error_handling::HandleErrorLayer;
use axum::extract::MatchedPath;
use axum::http::{HeaderValue, StatusCode};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;

#[derive(Clone)]
pub struct Server {
    pub config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Server { config }
    }

    pub async fn run(&self) -> std::io::Result<()> {
        let config = self.config.clone();

        // --- Logger ---
        // Initialisé en premier pour capturer tous les logs suivants
        logger::init(&config.server.environment);

        tracing::info!(
            environment = %config.server.environment,
            "Starting server"
        );

        // --- Database Pool ---
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&config.database.url);

        let pool = Pool::builder()
            .max_size(config.database.max_connections)
            .connection_timeout(Duration::from_secs(config.database.acquire_timeout))
            .idle_timeout(Some(Duration::from_secs(config.database.idle_timeout)))
            .max_lifetime(Some(Duration::from_secs(config.database.max_lifetime)))
            .build(manager)
            .await
            .expect("Failed to create database pool");

        tracing::info!("Database pool created successfully");

        // --- Migrations ---
        {
            use diesel::Connection;
            use diesel::pg::PgConnection;
            use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

            const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");
            let db_url = config.database.url.clone();

            tokio::task::spawn_blocking(move || {
                let mut conn =
                    PgConnection::establish(&db_url).expect("Failed to connect for migrations");
                conn.run_pending_migrations(MIGRATIONS)
                    .expect("Failed to run migrations");
            })
            .await
            .expect("Migration task panicked");

            tracing::info!("Migrations applied successfully");
        }

        let app_state = AppState {
            pool,
            config: Arc::new(config.clone()),
            rate_limit: RateLimitStore::new(
                10,  // 10 req/min sur les routes auth (login, register, refresh)
                120, // 120 req/min sur les routes protégées
            ),
        };

        // thread périodique qui vient nettoyer les caches des rate limiters pour éviter une croissance infinie en cas de nombreux clients uniques
        let rate_limit_clone = app_state.rate_limit.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600)); // nettoyage toutes les heures
            loop {
                interval.tick().await;
                rate_limit_clone.by_ip.retain_recent();
                rate_limit_clone.by_user.retain_recent();
                tracing::debug!("Rate limit cache cleaned");
            }
        });

        // --- CORS ---
        let cors = CorsLayer::new()
            .allow_origin(
                std::env::var("FRONTEND_URL")
                    .unwrap_or_else(|_| "http://localhost:5173".to_string())
                    .parse::<HeaderValue>()
                    .unwrap(),
            )
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ])
            .allow_credentials(true);

        // --- TraceLayer HTTP ---
        // Contrôlable via RUST_LOG=tower_http=debug
        let trace_layer = TraceLayer::new_for_http()
            .make_span_with(|request: &axum::http::Request<_>| {
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(|p| p.as_str())
                    .unwrap_or(request.uri().path());

                tracing::info_span!(
                    "http_request",
                    method  = %request.method(),
                    path    = %matched_path,
                    version = ?request.version(),
                )
            })
            .on_request(())
            .on_response(
                DefaultOnResponse::new()
                    .level(Level::INFO)
                    .include_headers(false),
            )
            .on_failure(DefaultOnFailure::new().level(Level::ERROR));

        // --- Router ---
        let app = Router::new()
            .merge(utoipa_swagger_ui::SwaggerUi::new("/swagger-ui").url(
                "/api-docs/openapi.json",
                {
                    use utoipa::OpenApi;
                    swagger::ApiDoc::openapi()
                },
            ))
            .nest("/api", create_router(app_state))
            .layer(cors)
            .layer(trace_layer)
            .layer(
                // Timeout global de 30s pour toutes les requêtes, avec gestion d'erreur personnalisée
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(|_: BoxError| async {
                        StatusCode::REQUEST_TIMEOUT
                    }))
                    .layer(TimeoutLayer::new(Duration::from_secs(30))),
            )
            .layer(RequestBodyLimitLayer::new(1024 * 1024)) // Limit de payload à 1MB pour éviter les abus
            .layer(CompressionLayer::new()); // Compression des réponses pour économiser la bande passante

        // --- Lancement ---
        let addr = format!("{}:{}", config.server.host, config.server.port);
        tracing::info!(address = %addr, "Server started 🚀");
        tracing::info!(openapi_url = %format!("http://{}/swagger-ui", addr), "OpenAPI docs available at");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::state::{DatabaseConfig, JwtConfig, ServerConfig};

    // ─── Helper ─────────────────────────────────────────────────────────────

    fn make_config(host: &str, port: u16, env: &str) -> Config {
        Config {
            server: ServerConfig {
                host: host.to_string(),
                port,
                environment: env.to_string(),
            },
            database: DatabaseConfig {
                url: "postgres://postgres:password@localhost:5432/app_db".to_string(),
                max_connections: 5,
                acquire_timeout: 30,
                idle_timeout: 600,
                max_lifetime: 1800,
            },
            jwt: JwtConfig {
                secret: "test_secret".to_string(),
                expiration: 86400,
                refresh_secret: "test_refresh_secret".to_string(),
                refresh_expiration: 604800,
            },
        }
    }

    // ─── Server::new ─────────────────────────────────────────────────────────

    #[test]
    fn test_server_new_stores_config() {
        let config = make_config("127.0.0.1", 8080, "development");
        let server = Server::new(config.clone());
        assert_eq!(server.config.server.host, "127.0.0.1");
        assert_eq!(server.config.server.port, 8080);
        assert_eq!(server.config.server.environment, "development");
    }

    #[test]
    fn test_server_clone() {
        let config = make_config("0.0.0.0", 3000, "production");
        let server = Server::new(config);
        let cloned = server.clone();
        assert_eq!(cloned.config.server.port, 3000);
        assert_eq!(cloned.config.server.environment, "production");
    }

    #[test]
    fn test_server_bind_address_format() {
        let config = make_config("127.0.0.1", 9090, "development");
        let server = Server::new(config);
        let addr = format!(
            "{}:{}",
            server.config.server.host, server.config.server.port
        );
        assert_eq!(addr, "127.0.0.1:9090");
        // Vérifie que l'adresse est parseable en SocketAddr
        assert!(
            addr.parse::<SocketAddr>().is_ok(),
            "adresse invalide : {}",
            addr
        );
    }

    #[test]
    fn test_server_bind_address_all_interfaces() {
        let config = make_config("0.0.0.0", 8080, "production");
        let server = Server::new(config);
        let addr = format!(
            "{}:{}",
            server.config.server.host, server.config.server.port
        );
        assert!(addr.parse::<SocketAddr>().is_ok());
    }

    // ─── Config values used at runtime ───────────────────────────────────────

    #[test]
    fn test_database_pool_config_values_are_positive() {
        let config = make_config("127.0.0.1", 8080, "development");
        assert!(config.database.max_connections > 0);
        assert!(config.database.acquire_timeout > 0);
        assert!(config.database.idle_timeout > 0);
        assert!(config.database.max_lifetime > 0);
    }

    #[test]
    fn test_idle_timeout_less_than_max_lifetime() {
        // idle_timeout doit être < max_lifetime pour éviter des connexions idle expirées
        // avant d'être récupérées par le pool
        let config = make_config("127.0.0.1", 8080, "development");
        assert!(
            config.database.idle_timeout < config.database.max_lifetime,
            "idle_timeout ({}) doit être < max_lifetime ({})",
            config.database.idle_timeout,
            config.database.max_lifetime
        );
    }

    #[test]
    fn test_jwt_expiration_less_than_refresh() {
        // Le JWT access token doit expirer avant le refresh token
        let config = make_config("127.0.0.1", 8080, "development");
        assert!(
            config.jwt.expiration < config.jwt.refresh_expiration,
            "jwt.expiration ({}) doit être < refresh_expiration ({})",
            config.jwt.expiration,
            config.jwt.refresh_expiration
        );
    }
}
