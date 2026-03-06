use crate::app::config::Config;
use crate::app::models::{AppState, Repositories, Services};
use crate::routes::swagger::ApiDoc;

use axum::{Router, serve};
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

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

        // --- Logger (Tracing) ---
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info,tower_http=debug".into()),
            )
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .init();

        tracing::info!("Starting server with configuration: {:#?}", config);

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
        // diesel-async ne gère pas les migrations, on utilise diesel_migrations
        {
            use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
            const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

            // Les migrations Diesel sont synchrones : on utilise un thread bloquant
            let db_url = config.database.url.clone();
            tokio::task::spawn_blocking(move || {
                use diesel::Connection;
                use diesel::pg::PgConnection;
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
        // let repositories = Arc::new(Repositories {
        //     user_repository: UserRepository,
        // });
        // let services = Services {
        //     user_service: UserService::new(Arc::clone(&repositories)),
        //     auth_service: AuthService::new(Arc::clone(&repositories)),
        // };
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
        // Note : allow_credentials(true) est incompatible avec allow_origin(Any)
        // En prod, remplace Any par ton domaine :
        // .allow_origin("https://ton-domaine.com".parse::<HeaderValue>().unwrap())
        // .allow_credentials(true)

        // --- Router ---
        let app = Router::new()
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .nest("/api", crate::routes::create_router(app_state))
            .layer(cors)
            .layer(TraceLayer::new_for_http());

        // --- Lancement ---
        let addr = format!("{}:{}", config.server.host, config.server.port);
        tracing::info!(
            "Server started at http://{} in {} mode 🚀",
            addr,
            config.server.environment
        );

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        serve(listener, app).await
    }
}
