//! Tests des sondes `/api/health/live` et `/api/health/ready` sur le router complet.
//!
//! Requiert `DATABASE_TEST_URL`.

#[allow(dead_code)] // seuls quelques helpers de `common` sont utilisés ici
mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// Router complet + drapeau d'arrêt partagé avec son état.
async fn app() -> (axum::Router, Arc<AtomicBool>) {
    let state = common::test_state().await;
    let shutting_down = state.shutting_down.clone();
    (api::app::build_app(state), shutting_down)
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let response = app
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn live_is_ok() {
    let (app, _) = app().await;
    let (status, body) = get(&app, "/api/health/live").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn ready_checks_the_database() {
    let (app, _) = app().await;
    let (status, body) = get(&app, "/api/health/ready").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn ready_returns_503_while_shutting_down() {
    let (app, shutting_down) = app().await;
    shutting_down.store(true, Ordering::Relaxed);

    let (status, body) = get(&app, "/api/health/ready").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "SERVICE_UNAVAILABLE");

    // La liveness reste OK : le process ne doit pas être redémarré pendant l'arrêt
    let (status, _) = get(&app, "/api/health/live").await;
    assert_eq!(status, StatusCode::OK);
}
