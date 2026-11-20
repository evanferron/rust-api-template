//! Démarrage et arrêt du serveur.
//!
//! Démarrage : logs → migrations → pool → état → écoute HTTP.
//! Arrêt (`SIGTERM` ou Ctrl+C) : plus aucune nouvelle connexion, les requêtes en cours
//! se terminent, dans la limite de `SHUTDOWN_TIMEOUT_SECS`.

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::watch;

use crate::app::telemetry;
use crate::common::middlewares::rate_limit::RateLimitStore;
use crate::common::state::AppState;
use crate::config::Config;
use crate::db;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Fréquence de purge des clés inactives des rate limiters.
const RATE_LIMIT_CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);

/// Démarre l'API et rend la main une fois le serveur arrêté proprement.
pub async fn run(config: Config) -> Result<(), BoxError> {
    // Initialisé en premier pour capturer tous les logs suivants
    telemetry::init(config.server.environment);
    tracing::info!(environment = %config.server.environment, "Starting server");

    // --- Base de données ---
    db::run_migrations(&config.database.url).await?;
    tracing::info!("Migrations applied");

    let pool = db::create_pool(&config.database).await?;
    tracing::info!("Database pool created");

    // --- État ---
    let state = AppState {
        pool,
        config: Arc::new(config.clone()),
        rate_limit: RateLimitStore::new(&config.rate_limit),
        shutting_down: Arc::new(AtomicBool::new(false)),
    };
    let shutting_down = state.shutting_down.clone();

    // `true` une fois le signal d'arrêt reçu
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let cleanup = tokio::spawn(clean_rate_limits(
        state.rate_limit.clone(),
        shutdown_rx.clone(),
    ));

    // --- Écoute HTTP ---
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(address = %addr, docs = %format!("http://{addr}/swagger-ui"), "Server started");

    tokio::spawn(async move {
        wait_for_signal().await;
        tracing::info!("Shutdown signal received, finishing in-flight requests");
        shutting_down.store(true, Ordering::Relaxed); // /health/ready → 503
        let _ = shutdown_tx.send(true);
    });

    let server = axum::serve(
        listener,
        super::build_app(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_requested(shutdown_rx.clone()))
    .into_future();

    let timeout = config.server.shutdown_timeout;
    let deadline = async move {
        shutdown_requested(shutdown_rx).await;
        tokio::time::sleep(timeout).await;
    };

    tokio::select! {
        result = server => result?,
        () = deadline => tracing::warn!(?timeout, "Shutdown timeout reached, remaining requests are aborted"),
    }

    cleanup.abort();
    tracing::info!("Server stopped");
    Ok(())
}

/// Se termine quand l'arrêt a été demandé.
async fn shutdown_requested(mut shutdown: watch::Receiver<bool>) {
    // Erreur = émetteur détruit : on considère aussi l'arrêt comme demandé
    let _ = shutdown.wait_for(|requested| *requested).await;
}

/// Attend `SIGTERM` (Docker, Kubernetes) ou Ctrl+C.
async fn wait_for_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

/// Purge périodiquement les clés inactives des rate limiters (sinon la mémoire croît
/// avec le nombre de clients distincts). S'arrête avec le serveur.
async fn clean_rate_limits(rate_limit: RateLimitStore, shutdown: watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(RATE_LIMIT_CLEANUP_INTERVAL);
    let stop = shutdown_requested(shutdown);
    tokio::pin!(stop);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                rate_limit.retain_recent();
                tracing::debug!("Rate limit cache cleaned");
            }
            () = &mut stop => break,
        }
    }
}
