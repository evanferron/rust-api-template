use std::collections::HashMap;

use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use cucumber::{World, gherkin::Step, given, then, when};
use tower::ServiceExt;

use crate::common::{get_test_app, reset_db, seed_user_and_login};

// ---------------------------------------------------------------------------
// World
// ---------------------------------------------------------------------------

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct UsersWorld {
    app: Router,
    /// Tokens indexés par email
    tokens: HashMap<String, String>,
    /// Email de l'utilisateur actif
    current_user: Option<String>,
    /// Dernier code HTTP reçu
    status: u16,
    /// Dernier body JSON reçu
    body: serde_json::Value,
}

impl UsersWorld {
    async fn new() -> Self {
        let app = get_test_app().await.clone();
        Self {
            app,
            tokens: HashMap::new(),
            current_user: None,
            status: 0,
            body: serde_json::Value::Null,
        }
    }

    // -----------------------------------------------------------------------
    // Helpers internes
    // -----------------------------------------------------------------------

    fn current_token(&self) -> String {
        let email = self
            .current_user
            .as_deref()
            .expect("Aucun utilisateur courant — appelle d'abord un step Given de login");
        self.tokens
            .get(email)
            .cloned()
            .unwrap_or_else(|| panic!("Token introuvable pour {email}"))
    }

    fn token_for(&self, email: &str) -> String {
        self.tokens
            .get(email)
            .cloned()
            .unwrap_or_else(|| panic!("Token introuvable pour {email}"))
    }

    async fn send(&mut self, req: Request<Body>) {
        let response = self.app.clone().oneshot(req).await.unwrap();
        self.status = response.status().as_u16();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        self.body = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
    }

    /// Récupère l'id d'un utilisateur par son email via GET /api/users.
    async fn fetch_user_id(&self, email: &str, token: &str) -> String {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/users")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let users: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        users
            .as_array()
            .unwrap()
            .iter()
            .find(|u| u["email"] == email)
            .unwrap_or_else(|| panic!("Utilisateur {email} introuvable dans la liste"))["id"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

// ---------------------------------------------------------------------------
// Conversion table Gherkin → HashMap
// ---------------------------------------------------------------------------

fn table_to_map(step: &Step) -> HashMap<String, String> {
    step.table()
        .expect("L'étape doit avoir une table de données")
        .rows
        .iter()
        .map(|row| (row[0].trim().to_string(), row[1].trim().to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

#[given("la base de données est réinitialisée")]
async fn reset_database(_world: &mut UsersWorld) {
    reset_db();
}

// ---------------------------------------------------------------------------
// Given — authentification
// ---------------------------------------------------------------------------

#[given(expr = "je suis connecté en tant que {string} avec le mot de passe {string}")]
#[when(expr = "je suis connecté en tant que {string} avec le mot de passe {string}")]
async fn given_logged_in_as(world: &mut UsersWorld, email: String, password: String) {
    // Inscription (idempotent — ignorée si l'utilisateur existe déjà)
    let reg_payload = serde_json::json!({
        "email":      email,
        "password":   password,
        "first_name": "Test",
        "last_name":  "User",
    });
    let _ = world
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(reg_payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let token = seed_user_and_login(&world.app, &email, &password).await;
    world.tokens.insert(email.clone(), token);
    world.current_user = Some(email);
}

// ---------------------------------------------------------------------------
// When — GET /api/users
// ---------------------------------------------------------------------------

#[when("je liste les utilisateurs")]
async fn when_list_users(world: &mut UsersWorld) {
    let token = world.current_token();
    let req = Request::builder()
        .method("GET")
        .uri("/api/users")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

#[when("je liste les utilisateurs sans token")]
async fn when_list_users_no_token(world: &mut UsersWorld) {
    let req = Request::builder()
        .method("GET")
        .uri("/api/users")
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// When — GET /api/users/:id
// ---------------------------------------------------------------------------

#[when("je récupère mon profil par son id")]
async fn when_get_own_profile(world: &mut UsersWorld) {
    let email = world
        .current_user
        .clone()
        .expect("Aucun utilisateur courant");
    let token = world.current_token();
    let user_id = world.fetch_user_id(&email, &token).await;

    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{user_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

#[when(expr = "je récupère l'utilisateur avec l'id {string}")]
async fn when_get_user_by_explicit_id(world: &mut UsersWorld, user_id: String) {
    let token = world.current_token();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/users/{user_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// When — PUT /api/users/:id
// ---------------------------------------------------------------------------

#[when(expr = "je modifie mon profil avec:")]
async fn when_update_own_profile(world: &mut UsersWorld, step: &Step) {
    let email = world
        .current_user
        .clone()
        .expect("Aucun utilisateur courant");
    let token = world.current_token();
    let user_id = world.fetch_user_id(&email, &token).await;

    let data = table_to_map(step);
    let payload = serde_json::to_value(&data).unwrap();

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/users/{user_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    world.send(req).await;
}

#[when(expr = "je modifie le profil de {string} avec:")]
async fn when_update_other_profile(world: &mut UsersWorld, target_email: String, step: &Step) {
    // Le token actif est celui de l'attaquant (current_user)
    let attacker_token = world.current_token();
    // On utilise le token de la cible pour récupérer son id
    let target_token = world.token_for(&target_email);
    let target_id = world.fetch_user_id(&target_email, &target_token).await;

    let data = table_to_map(step);
    let payload = serde_json::to_value(&data).unwrap();

    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/users/{target_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {attacker_token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// When — DELETE /api/users/:id
// ---------------------------------------------------------------------------

#[when("je supprime mon compte")]
async fn when_delete_own_account(world: &mut UsersWorld) {
    let email = world
        .current_user
        .clone()
        .expect("Aucun utilisateur courant");
    let token = world.current_token();
    let user_id = world.fetch_user_id(&email, &token).await;

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/users/{user_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

#[when(expr = "je supprime le compte de {string}")]
async fn when_delete_other_account(world: &mut UsersWorld, target_email: String) {
    let attacker_token = world.current_token();
    let target_token = world.token_for(&target_email);
    let target_id = world.fetch_user_id(&target_email, &target_token).await;

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/users/{target_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {attacker_token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// Then — assertions
// ---------------------------------------------------------------------------

#[then(expr = "le statut de la réponse est {int}")]
async fn then_status_is(world: &mut UsersWorld, expected: u16) {
    assert_eq!(
        world.status, expected,
        "Statut HTTP attendu : {expected}, obtenu : {}",
        world.status
    );
}

#[then(expr = "la réponse contient:")]
async fn then_response_contains(world: &mut UsersWorld, step: &Step) {
    for (key, expected_value) in table_to_map(step) {
        let actual = world.body[&key].as_str().unwrap_or("");
        assert_eq!(
            actual, expected_value,
            "body[\"{key}\"] attendu : \"{expected_value}\", obtenu : \"{actual}\""
        );
    }
}

#[then(expr = "la réponse est une liste de {int} utilisateur")]
async fn then_list_has_n_users(world: &mut UsersWorld, expected: usize) {
    let arr = world
        .body
        .as_array()
        .expect("La réponse devrait être un tableau JSON");
    assert_eq!(
        arr.len(),
        expected,
        "Nombre d'utilisateurs attendu : {expected}, obtenu : {}",
        arr.len()
    );
}
