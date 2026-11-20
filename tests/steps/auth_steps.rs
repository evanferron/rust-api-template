//! Steps de `auth.feature` : inscription, connexion, refresh.

use axum::http::Method;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use serde_json::Value;

use super::world::{ApiWorld, table_json};

/// Payload d'inscription : `first_name` / `last_name` par défaut si absents de la table.
fn register_payload(step: &Step) -> Value {
    let mut payload = table_json(step);
    for (field, default) in [("first_name", "Test"), ("last_name", "User")] {
        if payload.get(field).is_none() {
            payload[field] = default.into();
        }
    }
    payload
}

#[given(expr = "je suis enregistré avec:")]
#[when(expr = "je m'enregistre avec:")]
#[when(expr = "je m'enregistre à nouveau avec:")]
async fn register(world: &mut ApiWorld, step: &Step) {
    let payload = register_payload(step);
    world
        .send(Method::POST, "/api/auth/register", Some(payload), None)
        .await;
}

#[when(expr = "je me connecte avec:")]
async fn login(world: &mut ApiWorld, step: &Step) {
    let payload = table_json(step);
    world
        .send(Method::POST, "/api/auth/login", Some(payload), None)
        .await;
}

/// Inscription + connexion : le cookie de refresh est conservé dans le world.
#[given(expr = "je suis enregistré et connecté avec:")]
async fn registered_and_logged_in(world: &mut ApiWorld, step: &Step) {
    let payload = register_payload(step);
    let credentials = serde_json::json!({
        "email": payload["email"],
        "password": payload["password"],
    });
    world
        .send(Method::POST, "/api/auth/register", Some(payload), None)
        .await;
    world
        .send(Method::POST, "/api/auth/login", Some(credentials), None)
        .await;
}

#[when("j'appelle l'endpoint refresh avec le refresh_token cookie")]
async fn refresh_with_cookie(world: &mut ApiWorld) {
    assert!(world.cookie.is_some(), "aucun cookie de refresh reçu");
    world
        .send(Method::POST, "/api/auth/refresh", None, None)
        .await;
}

#[when("j'appelle l'endpoint refresh sans cookie")]
async fn refresh_without_cookie(world: &mut ApiWorld) {
    world.cookie = None;
    world
        .send(Method::POST, "/api/auth/refresh", None, None)
        .await;
}

#[then("la réponse contient un access_token")]
#[then("la réponse contient un nouvel access_token")]
async fn has_access_token(world: &mut ApiWorld) {
    let token = world.body["access_token"].as_str().unwrap_or_default();
    assert!(!token.is_empty(), "access_token absent de {}", world.body);
}
