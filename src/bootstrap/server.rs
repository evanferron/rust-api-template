use crate::bootstrap::config::Config;
use crate::bootstrap::models::AppState;
use crate::core::logger;
use crate::core::middlewares::rate_limit::RateLimitStore;
use axum::BoxError;
use axum::Router;
use axum::error_handling::HandleErrorLayer;
use axum::extract::MatchedPath;
use axum::http::{HeaderValue, StatusCode};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use std::net::SocketAddr;
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
            config: config.clone(),
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
                    crate::routes::swagger::ApiDoc::openapi()
                },
            ))
            .nest("/api", crate::routes::create_router(app_state))
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
