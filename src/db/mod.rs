//! Couche données : pool de connexions, migrations, schéma Diesel,
//! repository générique et un sous-module (model + repository) par table.
//!
//! Indépendante des modules HTTP : une table peut ne correspondre à aucun module.
//!
//! Tout passe par `diesel-async` (driver `tokio-postgres`, 100 % Rust) :
//! aucune dépendance à `libpq`, y compris pour les migrations.

pub mod post;
pub mod repository;
pub mod schema;
pub mod user;

use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::{AsyncConnection, AsyncMigrationHarness, AsyncPgConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

use crate::config::DatabaseConfig;

/// Migrations SQL de `./migrations`, embarquées dans le binaire à la compilation.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Crée le pool de connexions async.
pub async fn create_pool(
    config: &DatabaseConfig,
) -> Result<Pool<AsyncPgConnection>, diesel_async::pooled_connection::PoolError> {
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&config.url);

    Pool::builder()
        .max_size(config.max_connections)
        .connection_timeout(config.acquire_timeout)
        .idle_timeout(Some(config.idle_timeout))
        .max_lifetime(Some(config.max_lifetime))
        .build(manager)
        .await
}

/// Applique les migrations en attente.
pub async fn run_migrations(database_url: &str) -> Result<(), BoxError> {
    with_harness(database_url, |harness| {
        harness.run_pending_migrations(MIGRATIONS).map(|_| ())
    })
    .await
}

/// Annule toutes les migrations puis les réapplique (base vide). Réservé aux tests.
pub async fn reset_database(database_url: &str) -> Result<(), BoxError> {
    with_harness(database_url, |harness| {
        harness.revert_all_migrations(MIGRATIONS)?;
        harness.run_pending_migrations(MIGRATIONS).map(|_| ())
    })
    .await
}

/// Le harness de diesel_migrations est synchrone : il tourne dans un thread bloquant
/// et pilote la connexion async via le runtime tokio courant.
async fn with_harness<F>(database_url: &str, f: F) -> Result<(), BoxError>
where
    F: FnOnce(&mut AsyncMigrationHarness<AsyncPgConnection>) -> Result<(), BoxError>
        + Send
        + 'static,
{
    let conn = AsyncPgConnection::establish(database_url).await?;

    tokio::task::spawn_blocking(move || {
        let mut harness = AsyncMigrationHarness::new(conn);
        f(&mut harness)
    })
    .await?
}
