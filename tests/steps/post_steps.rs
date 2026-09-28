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
pub struct PostsWorld {
    app: Router,
    /// Tokens indexés par email — permet de switcher d'utilisateur dans un scénario
    tokens: HashMap<String, String>,
    /// Email de l'utilisateur actif pour les requêtes suivantes
    current_user: Option<String>,
    /// Dernier code HTTP reçu
    status: u16,
    /// Dernier body JSON reçu
    body: serde_json::Value,
    /// Id du dernier post créé (via `j'ai créé un post avec:`)
    last_post_id: Option<String>,
}

impl PostsWorld {
    async fn new() -> Self {
        let app = get_test_app().await.clone();
        Self {
            app,
            tokens: HashMap::new(),
            current_user: None,
            status: 0,
            body: serde_json::Value::Null,
            last_post_id: None,
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

    /// POST /api/posts — retourne l'id du post créé
    async fn api_create_post(&mut self, payload: serde_json::Value, token: Option<&str>) {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/posts")
            .header(header::CONTENT_TYPE, "application/json");

        if let Some(t) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }

        let req = builder.body(Body::from(payload.to_string())).unwrap();

        self.send(req).await;
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

fn map_to_post_payload(data: &HashMap<String, String>) -> serde_json::Value {
    let mut payload = serde_json::Map::new();

    if let Some(title) = data.get("title") {
        payload.insert("title".into(), serde_json::Value::String(title.clone()));
    }
    if let Some(content) = data.get("content") {
        payload.insert("content".into(), serde_json::Value::String(content.clone()));
    }
    if let Some(published) = data.get("published") {
        payload.insert(
            "published".into(),
            serde_json::Value::Bool(published == "true"),
        );
    }

    serde_json::Value::Object(payload)
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

#[given("la base de données est réinitialisée")]
async fn reset_database(_world: &mut PostsWorld) {
    reset_db();
}

// ---------------------------------------------------------------------------
// Given — authentification
// ---------------------------------------------------------------------------

#[given(expr = "je suis connecté en tant que {string} avec le mot de passe {string}")]
#[when(expr = "je suis connecté en tant que {string} avec le mot de passe {string}")]
async fn given_logged_in_as(world: &mut PostsWorld, email: String, password: String) {
    // Inscription si l'utilisateur n'existe pas encore
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

    // Login
    let token = seed_user_and_login(&world.app, &email, &password).await;
    world.tokens.insert(email.clone(), token);
    world.current_user = Some(email);
}

// ---------------------------------------------------------------------------
// Given — création de post en précondition
// ---------------------------------------------------------------------------

/// Crée un post ET stocke son id dans `last_post_id`.
/// Utilisé dans les préconditions (`Given`) sans écraser status/body du world.
#[given(expr = "j'ai créé un post avec:")]
async fn given_created_post(world: &mut PostsWorld, step: &Step) {
    let payload = map_to_post_payload(&table_to_map(step));
    let token = world.current_token();

    let response = world
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/posts")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status().as_u16(),
        201,
        "given_created_post: création du post échouée"
    );

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    world.last_post_id = json["id"].as_str().map(String::from);
}

/// Alias utilisé dans le scénario "lister les posts ne retourne que les siens"
/// où la création est une action observable (`And`).
#[given(expr = "je crée un post avec:")]
async fn given_create_post_inline(world: &mut PostsWorld, step: &Step) {
    given_created_post(world, step).await;
}

// ---------------------------------------------------------------------------
// When — POST /api/posts
// ---------------------------------------------------------------------------

#[when(expr = "je crée un post avec:")]
async fn when_create_post(world: &mut PostsWorld, step: &Step) {
    let payload = map_to_post_payload(&table_to_map(step));
    let token = world.current_token();
    world.api_create_post(payload, Some(&token.clone())).await;

    // Stocke l'id si le post a été créé avec succès
    if world.status == 201 {
        world.last_post_id = world.body["id"].as_str().map(String::from);
    }
}

#[when(expr = "je crée un post sans token avec:")]
async fn when_create_post_no_token(world: &mut PostsWorld, step: &Step) {
    let payload = map_to_post_payload(&table_to_map(step));
    world.api_create_post(payload, None).await;
}

// ---------------------------------------------------------------------------
// When — GET /api/posts
// ---------------------------------------------------------------------------

#[when("je liste mes posts")]
async fn when_list_own_posts(world: &mut PostsWorld) {
    let token = world.current_token();
    let req = Request::builder()
        .method("GET")
        .uri("/api/posts")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

#[when(expr = "je liste les posts de {string}")]
async fn when_list_posts_as(world: &mut PostsWorld, email: String) {
    let token = world
        .tokens
        .get(&email)
        .cloned()
        .unwrap_or_else(|| panic!("Token introuvable pour {email}"));
    let req = Request::builder()
        .method("GET")
        .uri("/api/posts")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// When — GET /api/posts/:id
// ---------------------------------------------------------------------------

#[when("je récupère le post par son id")]
async fn when_get_post_by_id(world: &mut PostsWorld) {
    let post_id = world
        .last_post_id
        .clone()
        .expect("Aucun post_id stocké dans le world");
    let token = world.current_token();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/posts/{post_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

#[when(expr = "je récupère le post avec l'id {string}")]
async fn when_get_post_by_explicit_id(world: &mut PostsWorld, post_id: String) {
    let token = world.current_token();
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/posts/{post_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// When — PUT /api/posts/:id
// ---------------------------------------------------------------------------

#[when(expr = "je modifie le post avec:")]
async fn when_update_post(world: &mut PostsWorld, step: &Step) {
    let post_id = world
        .last_post_id
        .clone()
        .expect("Aucun post_id stocké dans le world");
    let payload = map_to_post_payload(&table_to_map(step));
    let token = world.current_token();
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/posts/{post_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    world.send(req).await;
}

#[when(expr = "je modifie le post {string} avec:")]
async fn when_update_post_by_id(world: &mut PostsWorld, post_id: String, step: &Step) {
    let payload = map_to_post_payload(&table_to_map(step));
    let token = world.current_token();
    let req = Request::builder()
        .method("PUT")
        .uri(format!("/api/posts/{post_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// When — DELETE /api/posts/:id
// ---------------------------------------------------------------------------

#[when("je supprime le post")]
async fn when_delete_post(world: &mut PostsWorld) {
    let post_id = world
        .last_post_id
        .clone()
        .expect("Aucun post_id stocké dans le world");
    let token = world.current_token();
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/posts/{post_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// When — DELETE /api/users/:id (cascade)
// ---------------------------------------------------------------------------

#[when("je supprime mon compte")]
async fn when_delete_own_account(world: &mut PostsWorld) {
    let token = world.current_token();

    // Récupère la liste des users pour trouver son propre id
    let response = world
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
    let user_id = users[0]["id"]
        .as_str()
        .expect("Impossible de trouver l'id de l'utilisateur courant");

    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/users/{user_id}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    world.send(req).await;
}

// ---------------------------------------------------------------------------
// Then — assertions
// ---------------------------------------------------------------------------

#[then(expr = "le statut de la réponse est {int}")]
async fn then_status_is(world: &mut PostsWorld, expected: u16) {
    assert_eq!(
        world.status, expected,
        "Statut HTTP attendu : {expected}, obtenu : {}",
        world.status
    );
}

#[then(expr = "la réponse contient:")]
async fn then_response_contains(world: &mut PostsWorld, step: &Step) {
    for (key, expected_value) in table_to_map(step) {
        let actual = world.body[&key].as_str().unwrap_or("");
        assert_eq!(
            actual, expected_value,
            "body[\"{key}\"] attendu : \"{expected_value}\", obtenu : \"{actual}\""
        );
    }
}

#[then(expr = "la réponse contient un champ {string}")]
async fn then_response_has_field(world: &mut PostsWorld, field: String) {
    assert!(
        world.body.get(&field).is_some(),
        "Le champ \"{field}\" est absent de la réponse"
    );
}

#[then(expr = "le champ {string} de la réponse vaut {string}")]
async fn then_field_equals(world: &mut PostsWorld, field: String, expected: String) {
    let actual = world.body[&field].to_string();
    // On compare en tenant compte des booléens JSON (sans guillemets)
    assert_eq!(
        actual, expected,
        "body[\"{field}\"] attendu : \"{expected}\", obtenu : \"{actual}\""
    );
}

#[then("le mot de passe n'apparaît pas dans la réponse")]
async fn then_no_password(world: &mut PostsWorld) {
    assert!(world.body.get("password").is_none());
    assert!(world.body.get("password_hash").is_none());
}

#[then("la réponse est une liste vide")]
async fn then_empty_list(world: &mut PostsWorld) {
    let arr = world
        .body
        .as_array()
        .expect("La réponse devrait être un tableau JSON");
    assert_eq!(arr.len(), 0, "La liste devrait être vide");
}

#[then(expr = "la réponse contient exactement {int} posts")]
async fn then_list_has_n_posts(world: &mut PostsWorld, expected: usize) {
    let arr = world
        .body
        .as_array()
        .expect("La réponse devrait être un tableau JSON");
    assert_eq!(arr.len(), expected, "Nombre de posts attendu : {expected}");
}

#[then(expr = "aucun post de la liste n'a le titre {string}")]
async fn then_no_post_with_title(world: &mut PostsWorld, forbidden_title: String) {
    let arr = world
        .body
        .as_array()
        .expect("La réponse devrait être un tableau JSON");
    for post in arr {
        assert_ne!(
            post["title"].as_str().unwrap_or(""),
            forbidden_title,
            "Un post avec le titre \"{forbidden_title}\" ne devrait pas être visible"
        );
    }
}

#[then("le post n'est plus accessible")]
async fn then_post_not_accessible(world: &mut PostsWorld) {
    let post_id = world
        .last_post_id
        .clone()
        .expect("Aucun post_id stocké dans le world");
    let token = world.current_token();

    let response = world
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/posts/{post_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status().as_u16(),
        404,
        "Le post devrait retourner 404 après suppression"
    );
}
