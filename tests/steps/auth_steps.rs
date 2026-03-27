use std::collections::HashMap;

use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use cucumber::{World, gherkin::Step, given, then, when};
use tower::ServiceExt;

use crate::common::{get_test_app, reset_db};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct AuthWorld {
    app: Router,
    status: u16,
    body: serde_json::Value,
    cookie: Option<String>,
}

impl AuthWorld {
    async fn new() -> Self {
        let app = get_test_app().await.clone();
        Self {
            app,
            status: 0,
            body: serde_json::Value::Null,
            cookie: None,
        }
    }

    /// Exécute une requête POST JSON et met à jour l'état du world.
    async fn post_json(&mut self, uri: &str, payload: serde_json::Value) {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        self.status = response.status().as_u16();

        // Conserve le cookie Set-Cookie s'il est présent (refresh token, etc.)
        if let Some(cookie_val) = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
        {
            self.cookie = Some(cookie_val.to_string());
        }

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        self.body = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convertit une table Gherkin à deux colonnes (clé | valeur) en HashMap.
fn table_to_map(step: &Step) -> HashMap<String, String> {
    step.table()
        .expect("L'étape doit avoir une table de données")
        .rows
        .iter()
        .map(|row| (row[0].trim().to_string(), row[1].trim().to_string()))
        .collect()
}

/// Construit un payload de registration à partir d'une table.
/// Les champs first_name / last_name ont des valeurs par défaut si absents.
fn build_register_payload(data: &HashMap<String, String>) -> serde_json::Value {
    serde_json::json!({
        "email":      data["email"],
        "password":   data["password"],
        "first_name": data.get("first_name").map(String::as_str).unwrap_or("Test"),
        "last_name":  data.get("last_name").map(String::as_str).unwrap_or("User"),
    })
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

#[given("la base de données est réinitialisée")]
async fn reset_database(_world: &mut AuthWorld) {
    reset_db();
}

// ---------------------------------------------------------------------------
// Register – Given / When
// ---------------------------------------------------------------------------

/// Enregistre un utilisateur sans mettre à jour le statut/body du world
/// (utilisé dans les préconditions Given).
#[given(expr = "je suis enregistré avec:")]
async fn given_registered_with(world: &mut AuthWorld, step: &Step) {
    let payload = build_register_payload(&table_to_map(step));
    world.post_json("/api/auth/register", payload).await;
}

/// Premier appel à register (met à jour le statut/body du world).
#[when(expr = "je m'enregistre avec:")]
async fn when_register_with(world: &mut AuthWorld, step: &Step) {
    let payload = build_register_payload(&table_to_map(step));
    world.post_json("/api/auth/register", payload).await;
}

/// Deuxième appel à register dans le même scénario (ex. : email dupliqué).
#[when(expr = "je m'enregistre à nouveau avec:")]
async fn when_register_again_with(world: &mut AuthWorld, step: &Step) {
    let payload = build_register_payload(&table_to_map(step));
    world.post_json("/api/auth/register", payload).await;
}

// ---------------------------------------------------------------------------
// Login – Given / When
// ---------------------------------------------------------------------------

#[when(expr = "je me connecte avec:")]
async fn when_login_with(world: &mut AuthWorld, step: &Step) {
    let data = table_to_map(step);
    let payload = serde_json::json!({
        "email":    data["email"],
        "password": data["password"],
    });
    world.post_json("/api/auth/login", payload).await;
}

// ---------------------------------------------------------------------------
// Refresh – Given / When
// ---------------------------------------------------------------------------

/// Enregistre l'utilisateur et effectue le login afin de stocker le cookie
/// de refresh token dans le world.
#[given(expr = "je suis enregistré et connecté avec:")]
async fn given_registered_and_logged_in(world: &mut AuthWorld, step: &Step) {
    let data = table_to_map(step);

    // 1. Inscription
    let reg_payload = build_register_payload(&data);
    world.post_json("/api/auth/register", reg_payload).await;

    // 2. Connexion – le cookie Set-Cookie est stocké dans world.cookie
    let login_payload = serde_json::json!({
        "email":    data["email"],
        "password": data["password"],
    });
    world.post_json("/api/auth/login", login_payload).await;
}

#[when("j'appelle l'endpoint refresh avec le refresh_token cookie")]
async fn when_refresh_with_cookie(world: &mut AuthWorld) {
    let cookie = world
        .cookie
        .clone()
        .expect("Aucun cookie de refresh stocké dans le world");

    let response = world
        .app
        .clone()
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

    world.status = response.status().as_u16();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
}

#[when("j'appelle l'endpoint refresh sans cookie")]
async fn when_refresh_without_cookie(world: &mut AuthWorld) {
    let response = world
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    world.status = response.status().as_u16();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    world.body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
}

// ---------------------------------------------------------------------------
// Then – assertions
// ---------------------------------------------------------------------------

#[then(expr = "le statut de la réponse est {int}")]
async fn then_status_is(world: &mut AuthWorld, expected: u16) {
    assert_eq!(
        world.status, expected,
        "Statut HTTP attendu : {expected}, obtenu : {}",
        world.status
    );
}

#[then(expr = "la réponse contient:")]
async fn then_response_contains(world: &mut AuthWorld, step: &Step) {
    for (key, expected_value) in table_to_map(step) {
        let actual = world.body[&key].as_str().unwrap_or("");
        assert_eq!(
            actual, expected_value,
            "body[\"{key}\"] attendu : \"{expected_value}\", obtenu : \"{actual}\""
        );
    }
}

#[then("le hash du mot de passe n'apparaît pas dans la réponse")]
async fn then_no_password_in_response(world: &mut AuthWorld) {
    assert!(
        world.body.get("password_hash").is_none(),
        "Le champ password_hash ne doit pas figurer dans la réponse"
    );
    assert!(
        world.body.get("password").is_none(),
        "Le champ password ne doit pas figurer dans la réponse"
    );
}

#[then("la réponse contient un access_token")]
async fn then_response_has_access_token(world: &mut AuthWorld) {
    let token = world.body["access_token"].as_str().unwrap_or("");
    assert!(
        !token.is_empty(),
        "access_token manquant ou vide dans la réponse"
    );
}

#[then("la réponse contient un nouvel access_token")]
async fn then_response_has_new_access_token(world: &mut AuthWorld) {
    let token = world.body["access_token"].as_str().unwrap_or("");
    assert!(
        !token.is_empty(),
        "Nouvel access_token manquant ou vide dans la réponse"
    );
}
