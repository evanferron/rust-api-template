//! Assemblage de l'application : routes des modules, couches HTTP et documentation OpenAPI.
//!
//! [`build_app`] est utilisé par le serveur **et** par les tests : ils exécutent exactement
//! le même router.

mod openapi;
pub mod server;
pub mod telemetry;

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{DefaultBodyLimit, MatchedPath};
use axum::http::{HeaderValue, Method, Request, header};
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::common::error;
use crate::common::middlewares::request_id::{X_REQUEST_ID, request_id};
use crate::common::state::AppState;
use crate::modules::{auth, health, post, user};

/// Taille maximale d'un corps de requête.
const BODY_LIMIT: usize = 1024 * 1024;
/// Durée maximale de traitement d'une requête.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Construit le router complet : `/api/*`, Swagger UI, fallbacks et couches.
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        .nest("/api", api_routes(state))
        // Routes et méthodes inconnues → 404 / 405 au format JSON
        .fallback(error::route_not_found)
        .method_not_allowed_fallback(error::method_not_allowed)
        // Couches appliquées de la plus interne (haut) à la plus externe (bas)
        .layer(DefaultBodyLimit::max(BODY_LIMIT)) // → PAYLOAD_TOO_LARGE
        .layer(cors())
        .layer(CompressionLayer::new())
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(error::handle_layer_error)) // → REQUEST_TIMEOUT
                .layer(TimeoutLayer::new(REQUEST_TIMEOUT)),
        )
        .layer(CatchPanicLayer::custom(error::handle_panic)) // panic → INTERNAL
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_span)
                .on_request(())
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
        )
        .layer(axum::middleware::from_fn(request_id)) // la plus externe : tout est tracé avec un id
}

/// Routes des modules, montées sous `/api`.
fn api_routes(state: AppState) -> Router {
    Router::new()
        .merge(health::routes())
        .merge(auth::routes(state.clone()))
        .merge(user::routes(state.clone()))
        .merge(post::routes(state))
}

fn cors() -> CorsLayer {
    let origin = std::env::var("FRONTEND_URL")
        .unwrap_or_else(|_| "http://localhost:5173".to_string())
        .parse::<HeaderValue>()
        .expect("FRONTEND_URL must be a valid header value");

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        .allow_credentials(true)
}

/// Span de log d'une requête (méthode, route, request_id). Contrôlable via `RUST_LOG=tower_http=debug`.
fn make_span(request: &Request<Body>) -> tracing::Span {
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
}
