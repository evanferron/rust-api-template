mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::{reset_db, seed_user_and_login};
use tower::ServiceExt;

use crate::common::get_test_app;

// ---------------------------------------------------------------------------
// POST /api/auth/register
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_register_success() {
    reset_db();
    let app = get_test_app().await.clone();

    let body = serde_json::json!({
        "email": "register@test.com",
        "password": "securepass",
        "first_name": "Evan",
        "last_name": "Ferron"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Lis le body AVANT d'asserter le status
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&bytes);

    println!("Status: {}", status);
    println!("Body: {}", body_str);

    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json["email"], "register@test.com");
    assert_eq!(json["first_name"], "Evan");
    // Le hash du mot de passe ne doit jamais apparaître dans la réponse
    assert!(json.get("password_hash").is_none());
    assert!(json.get("password").is_none());
}

#[tokio::test]
async fn test_register_invalid_email() {
    reset_db();
    let app = get_test_app().await.clone();

    let body = serde_json::json!({
        "email": "not-an-email",
        "password": "securepass",
        "first_name": "Evan",
        "last_name": "Ferron"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_password_too_short() {
    reset_db();
    let app = get_test_app().await.clone();

    let body = serde_json::json!({
        "email": "valid@test.com",
        "password": "short",
        "first_name": "Evan",
        "last_name": "Ferron"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_register_duplicate_email() {
    reset_db();
    let app = get_test_app().await.clone();

    let body = serde_json::json!({
        "email": "duplicate@test.com",
        "password": "securepass",
        "first_name": "Evan",
        "last_name": "Ferron"
    });

    // Première inscription
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Deuxième inscription avec le même email
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
}

// ---------------------------------------------------------------------------
// POST /api/auth/login
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_login_success() {
    reset_db();
    let app = get_test_app().await.clone();

    let token = seed_user_and_login(&app, "login@test.com", "securepass").await;
    assert!(!token.is_empty());
}

#[tokio::test]
async fn test_login_wrong_password() {
    reset_db();
    let app = get_test_app().await.clone();

    // Crée l'utilisateur
    seed_user_and_login(&app, "user@test.com", "correctpass").await;

    let body = serde_json::json!({
        "email": "user@test.com",
        "password": "wrongpass"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_unknown_email() {
    reset_db();
    let app = get_test_app().await.clone();

    let body = serde_json::json!({
        "email": "ghost@test.com",
        "password": "anypass"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// POST /api/auth/refresh
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_refresh_with_valid_cookie() {
    reset_db();
    let app = get_test_app().await.clone();

    // Login pour récupérer le cookie
    let login_body = serde_json::json!({
        "email": "refresh@test.com",
        "password": "securepass"
    });

    seed_user_and_login(&app, "refresh@test.com", "securepass").await;

    // Récupère le cookie depuis la réponse login
    let login_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(login_body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let cookie = login_response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Refresh avec le cookie
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(!json["access_token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_refresh_without_cookie() {
    reset_db();
    let app = get_test_app().await.clone();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
