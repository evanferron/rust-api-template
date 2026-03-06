use axum::{Router, serve};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::app::config::Config;
use crate::app::models::{AppState, Repositories, Services};
use crate::core::logger;

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

        // --- Dependency Injection ---
        let repositories = Arc::new(Repositories {});
        let services = Services {};

        let app_state = AppState {
            pool,
            config: config.clone(),
            services,
            repositories,
        };

        // --- CORS ---
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::PUT,
                axum::http::Method::DELETE,
            ])
            .allow_headers([
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
            ]);

        // --- TraceLayer HTTP ---
        // Contrôlable via RUST_LOG=tower_http=debug
        let trace_layer = TraceLayer::new_for_http()
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
                    crate::routes::swagger::ApiDoc::openapi()
                },
            ))
            .nest("/api", crate::routes::create_router(app_state))
            .layer(cors)
            .layer(trace_layer);

        // --- Lancement ---
        let addr = format!("{}:{}", config.server.host, config.server.port);
        tracing::info!(address = %addr, "Server started 🚀");
        tracing::info!(openapi_url = %format!("http://{}/swagger-ui", addr), "OpenAPI docs available at");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        serve(listener, app).await
    }
}
