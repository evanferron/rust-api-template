mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::{reset_db, seed_user_and_login};
use tower::ServiceExt;

use crate::common::get_test_app;

// ---------------------------------------------------------------------------
// GET /api/users
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_all_users_authenticated() {
    reset_db();
    let app = get_test_app().await.clone();

    let token = seed_user_and_login(&app, "user@test.com", "securepass").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
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
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_get_all_users_unauthenticated() {
    reset_db();
    let app = get_test_app().await.clone();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// GET /api/users/:id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_user_by_id() {
    reset_db();
    let app = get_test_app().await.clone();

    let token = seed_user_and_login(&app, "getbyid@test.com", "securepass").await;

    // Récupère la liste pour avoir l'UUID
    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let users: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let user_id = users[0]["id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/users/{}", user_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
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
    assert_eq!(json["email"], "getbyid@test.com");
}

#[tokio::test]
async fn test_get_user_by_id_not_found() {
    reset_db();
    let app = get_test_app().await.clone();

    let token = seed_user_and_login(&app, "user@test.com", "securepass").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/00000000-0000-0000-0000-000000000000")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// PUT /api/users/:id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_update_own_profile() {
    reset_db();
    let app = get_test_app().await.clone();

    let token = seed_user_and_login(&app, "update@test.com", "securepass").await;

    // Récupère l'UUID
    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let users: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let user_id = users[0]["id"].as_str().unwrap();

    let body = serde_json::json!({ "first_name": "Updated" });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/users/{}", user_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["first_name"], "Updated");
}

#[tokio::test]
async fn test_update_other_user_forbidden() {
    reset_db();
    let app = get_test_app().await.clone();

    // Deux utilisateurs
    let token_a = seed_user_and_login(&app, "user_a@test.com", "securepass").await;
    let token_b = seed_user_and_login(&app, "user_b@test.com", "securepass").await;

    // Récupère l'ID de user_a
    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let users: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let user_a_id = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "user_a@test.com")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    // user_b essaie de modifier user_a
    let body = serde_json::json!({ "first_name": "Hacked" });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/users/{}", user_a_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token_b))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// DELETE /api/users/:id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_own_account() {
    reset_db();
    let app = get_test_app().await.clone();

    let token = seed_user_and_login(&app, "delete@test.com", "securepass").await;

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let users: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let user_id = users[0]["id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/users/{}", user_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_other_user_forbidden() {
    reset_db();
    let app = get_test_app().await.clone();

    let token_a = seed_user_and_login(&app, "victim@test.com", "securepass").await;
    let token_b = seed_user_and_login(&app, "attacker@test.com", "securepass").await;

    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let users: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let victim_id = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "victim@test.com")
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/users/{}", victim_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token_b))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
