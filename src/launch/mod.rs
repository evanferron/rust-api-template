pub mod router;
mod swagger;

use crate::core::errors;
use crate::core::logger;
use crate::core::middlewares::rate_limit::RateLimitStore;
use crate::core::middlewares::request_id::{X_REQUEST_ID, request_id};
use crate::infra::config::Config;
use crate::infra::database;
use crate::infra::state::AppState;
use crate::launch::router::create_router;
use axum::Router;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{DefaultBodyLimit, MatchedPath};
use axum::http::HeaderValue;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
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

        // --- Database ---
        database::run_migrations(&config.database.url)
            .await
            .expect("Failed to run migrations");
        tracing::info!("Migrations applied successfully");

        let pool = database::create_pool(&config.database)
            .await
            .expect("Failed to create database pool");
        tracing::info!("Database pool created successfully");

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
                let request_id = request
                    .headers()
                    .get(X_REQUEST_ID)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default();

                tracing::info_span!(
                    "http_request",
                    method     = %request.method(),
                    path       = %matched_path,
                    version    = ?request.version(),
                    request_id = %request_id,
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
            // Routes et méthodes inconnues → 404 / 405 au format JSON
            .fallback(errors::route_not_found)
            .method_not_allowed_fallback(errors::method_not_allowed)
            // Couches appliquées de la plus interne (haut) à la plus externe (bas)
            .layer(DefaultBodyLimit::max(1024 * 1024)) // 1 MB → PAYLOAD_TOO_LARGE
            .layer(cors)
            .layer(CompressionLayer::new())
            .layer(
                // Timeout global de 30s → REQUEST_TIMEOUT
                ServiceBuilder::new()
                    .layer(HandleErrorLayer::new(errors::handle_layer_error))
                    .layer(TimeoutLayer::new(Duration::from_secs(30))),
            )
            .layer(CatchPanicLayer::custom(errors::handle_panic)) // panic → INTERNAL
            .layer(trace_layer)
            .layer(axum::middleware::from_fn(request_id)); // la plus externe : tout est tracé avec un id

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
