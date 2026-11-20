//! Assemblage de l'application : routes des modules, couches HTTP et documentation OpenAPI.
//!
//! [`build_app`] est utilisé par le serveur **et** par les tests : ils exécutent exactement
//! le même router.

mod openapi;
pub mod server;
pub mod telemetry;

use axum::Router;
use axum::body::Body;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{DefaultBodyLimit, MatchedPath};
use axum::http::{Method, Request, header};
use tower::ServiceBuilder;
use tower::timeout::TimeoutLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::common::error;
use crate::common::middlewares::rate_limit::rate_limit;
use crate::common::middlewares::request_id::{X_REQUEST_ID, request_id};
use crate::common::state::AppState;
use crate::config::HttpConfig;
use crate::modules::{auth, health, post, user};

/// Routes des modules, montées sous `/api`, avec leur documentation OpenAPI.
///
/// **Ajouter un module** : une ligne `.merge(<module>::routes())` ici.
fn api_router() -> OpenApiRouter<AppState> {
    let modules = OpenApiRouter::new()
        .merge(health::routes())
        .merge(auth::routes())
        .merge(user::routes())
        .merge(post::routes());

    OpenApiRouter::with_openapi(openapi::ApiDoc::openapi()).nest("/api", modules)
}

/// Documentation OpenAPI complète (servie sur `/api-docs/openapi.json`).
pub fn openapi_doc() -> utoipa::openapi::OpenApi {
    api_router().into_openapi()
}

/// Construit le router complet : `/api/*`, Swagger UI, fallbacks et couches.
pub fn build_app(state: AppState) -> Router {
    let http = state.config.http.clone();
    let (api, doc) = api_router().split_for_parts();
    let api = api
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit,
        )) // par user ou IP
        .with_state(state);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", doc))
        .merge(api)
        // Routes et méthodes inconnues → 404 / 405 au format JSON
        .fallback(error::route_not_found)
        .method_not_allowed_fallback(error::method_not_allowed)
        // Couches appliquées de la plus interne (haut) à la plus externe (bas)
        .layer(DefaultBodyLimit::max(http.body_limit)) // → PAYLOAD_TOO_LARGE
        .layer(cors(&http))
        .layer(CompressionLayer::new())
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(error::handle_layer_error)) // → REQUEST_TIMEOUT
                .layer(TimeoutLayer::new(http.request_timeout)),
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

fn cors(http: &HttpConfig) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(http.cors_allowed_origins.clone())
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
