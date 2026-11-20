//! Démarrage du serveur : logs, migrations, pool de connexions, état, écoute HTTP.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::app::telemetry;
use crate::common::middlewares::rate_limit::RateLimitStore;
use crate::common::state::AppState;
use crate::config::Config;
use crate::db;

/// Démarre l'API : logs, migrations, pool, puis serveur HTTP.
pub async fn run(config: Config) -> std::io::Result<()> {
    // --- Logger ---
    // Initialisé en premier pour capturer tous les logs suivants
    telemetry::init(&config.server.environment);

    tracing::info!(
        environment = %config.server.environment,
        "Starting server"
    );

    // --- Database ---
    db::run_migrations(&config.database.url)
        .await
        .expect("Failed to run migrations");
    tracing::info!("Migrations applied successfully");

    let pool = db::create_pool(&config.database)
        .await
        .expect("Failed to create database pool");
    tracing::info!("Database pool created successfully");

    let app_state = AppState {
        pool,
        config: Arc::new(config.clone()),
        rate_limit: RateLimitStore::new(
            10,  // 10 req/min par IP sur login / register / refresh
            120, // 120 req/min par utilisateur (ou par IP si anonyme) sur toute l'API
        ),
    };

    // thread périodique qui vient nettoyer les caches des rate limiters pour éviter une croissance infinie en cas de nombreux clients uniques
    let rate_limit_clone = app_state.rate_limit.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // nettoyage toutes les heures
        loop {
            interval.tick().await;
            rate_limit_clone.retain_recent();
            tracing::debug!("Rate limit cache cleaned");
        }
    });

    let app = super::build_app(app_state);

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
