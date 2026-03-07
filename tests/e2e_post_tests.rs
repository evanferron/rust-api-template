mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use common::{reset_db, seed_user_and_login};
use tower::ServiceExt;

use crate::common::get_test_app;

// ---------------------------------------------------------------------------
// Helpers locaux
// ---------------------------------------------------------------------------

/// Crée un post via l'API et retourne le JSON de la réponse.
async fn create_post(
    app: &axum::Router,
    token: &str,
    title: &str,
    content: &str,
    published: Option<bool>,
) -> serde_json::Value {
    let body = serde_json::json!({
        "title": title,
        "content": content,
        "published": published,
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/posts")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "create_post helper failed"
    );

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// POST /api/posts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_post_success() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let body = serde_json::json!({
        "title": "Mon premier post",
        "content": "Contenu du post",
        "published": false,
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/posts")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["title"], "Mon premier post");
    assert_eq!(json["content"], "Contenu du post");
    assert_eq!(json["published"], false);
    assert!(json["id"].as_str().is_some());
    assert!(json["user_id"].as_str().is_some());
    // Le password ne doit jamais fuiter dans une réponse post
    assert!(json.get("password").is_none());
}

#[tokio::test]
async fn test_create_post_published_default_false() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    // published absent → doit valoir false par défaut
    let body = serde_json::json!({
        "title": "Post sans published",
        "content": "Contenu",
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/posts")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["published"], false);
}

#[tokio::test]
async fn test_create_post_empty_title_fails() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let body = serde_json::json!({
        "title": "",
        "content": "Contenu valide",
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/posts")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_post_unauthenticated() {
    reset_db();
    let app = get_test_app().await.clone();

    let body = serde_json::json!({
        "title": "Post sans token",
        "content": "Contenu",
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/posts")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// GET /api/posts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_all_posts_empty() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/posts")
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
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_get_all_posts_only_own() {
    reset_db();
    let app = get_test_app().await.clone();

    let token_a = seed_user_and_login(&app, "user_a@test.com", "securepass").await;
    let token_b = seed_user_and_login(&app, "user_b@test.com", "securepass").await;

    // user_a crée 2 posts, user_b en crée 1
    create_post(&app, &token_a, "Post A1", "Contenu", None).await;
    create_post(&app, &token_a, "Post A2", "Contenu", None).await;
    create_post(&app, &token_b, "Post B1", "Contenu", None).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/posts")
                .header(header::AUTHORIZATION, format!("Bearer {}", token_a))
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

    // user_a ne voit que ses 2 posts, pas celui de user_b
    assert_eq!(json.as_array().unwrap().len(), 2);
    for post in json.as_array().unwrap() {
        assert_ne!(post["title"], "Post B1");
    }
}

// ---------------------------------------------------------------------------
// GET /api/posts/:id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_post_by_id_success() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let post = create_post(&app, &token, "Mon post", "Contenu", None).await;
    let post_id = post["id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/posts/{}", post_id))
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

    assert_eq!(json["id"], post_id);
    assert_eq!(json["title"], "Mon post");
}

#[tokio::test]
async fn test_get_post_by_id_not_found() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/posts/00000000-0000-0000-0000-000000000000")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// PUT /api/posts/:id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_update_own_post() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let post = create_post(&app, &token, "Titre original", "Contenu original", None).await;
    let post_id = post["id"].as_str().unwrap();

    let body = serde_json::json!({
        "title": "Titre modifié",
        "published": true,
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/posts/{}", post_id))
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

    assert_eq!(json["title"], "Titre modifié");
    assert_eq!(json["published"], true);
    // Le contenu non fourni reste inchangé
    assert_eq!(json["content"], "Contenu original");
}

#[tokio::test]
async fn test_update_other_user_post_forbidden() {
    reset_db();
    let app = get_test_app().await.clone();

    let token_a = seed_user_and_login(&app, "owner@test.com", "securepass").await;
    let token_b = seed_user_and_login(&app, "hacker@test.com", "securepass").await;

    let post = create_post(&app, &token_a, "Post de A", "Contenu", None).await;
    let post_id = post["id"].as_str().unwrap();

    let body = serde_json::json!({ "title": "Post hacké" });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/posts/{}", post_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token_b))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_update_post_not_found() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let body = serde_json::json!({ "title": "Peu importe" });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/posts/00000000-0000-0000-0000-000000000000")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// DELETE /api/posts/:id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_own_post() {
    reset_db();
    let app = get_test_app().await.clone();
    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;

    let post = create_post(&app, &token, "Post à supprimer", "Contenu", None).await;
    let post_id = post["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/posts/{}", post_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // Vérifie que le post n'est plus accessible
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/posts/{}", post_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_other_user_post_forbidden() {
    reset_db();
    let app = get_test_app().await.clone();

    let token_a = seed_user_and_login(&app, "owner@test.com", "securepass").await;
    let token_b = seed_user_and_login(&app, "hacker@test.com", "securepass").await;

    let post = create_post(&app, &token_a, "Post de A", "Contenu", None).await;
    let post_id = post["id"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/posts/{}", post_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token_b))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// Cascade — DELETE user supprime ses posts
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_user_cascades_to_posts() {
    reset_db();
    let app = get_test_app().await.clone();

    let token = seed_user_and_login(&app, "author@test.com", "securepass").await;
    let post = create_post(&app, &token, "Post orphelin", "Contenu", None).await;
    let post_id = post["id"].as_str().unwrap().to_string();

    // Récupère l'ID de l'utilisateur
    let users_response = app
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

    let bytes = axum::body::to_bytes(users_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let users: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let user_id = users[0]["id"].as_str().unwrap();

    // Supprime l'utilisateur
    app.clone()
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

    // Crée un autre user pour vérifier que le post n'existe plus
    let token_other = seed_user_and_login(&app, "other@test.com", "securepass").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/posts/{}", post_id))
                .header(header::AUTHORIZATION, format!("Bearer {}", token_other))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // ON DELETE CASCADE — le post doit avoir été supprimé avec l'utilisateur
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
