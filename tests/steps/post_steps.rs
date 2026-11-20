//! Steps de `post.feature`.

use axum::http::Method;
use cucumber::gherkin::Step;
use cucumber::{given, then, when};
use serde_json::Value;

use super::world::{ApiWorld, table_json};

/// Crée un post avec l'utilisateur courant et retient son id dans `last_id`.
#[given(expr = "j'ai créé un post avec:")]
#[given(expr = "je crée un post avec:")]
#[when(expr = "je crée un post avec:")]
async fn create_post(world: &mut ApiWorld, step: &Step) {
    world
        .send_as_current(Method::POST, "/api/posts", Some(table_json(step)))
        .await;
    if world.status == 201 {
        world.last_id = world.body["id"].as_str().map(String::from);
    }
}

#[when(expr = "je crée un post sans token avec:")]
async fn create_post_without_token(world: &mut ApiWorld, step: &Step) {
    world
        .send(Method::POST, "/api/posts", Some(table_json(step)), None)
        .await;
}

#[when("je liste mes posts")]
async fn list_own_posts(world: &mut ApiWorld) {
    world.send_as_current(Method::GET, "/api/posts", None).await;
}

#[when(expr = "je liste les posts de {string}")]
async fn list_posts_of(world: &mut ApiWorld, email: String) {
    let token = world.token_of(&email);
    world
        .send(Method::GET, "/api/posts", None, Some(token))
        .await;
}

#[when("je récupère le post par son id")]
async fn get_last_post(world: &mut ApiWorld) {
    let id = world.last_id();
    world
        .send_as_current(Method::GET, &format!("/api/posts/{id}"), None)
        .await;
}

#[when(expr = "je récupère le post avec l'id {string}")]
async fn get_post_by_id(world: &mut ApiWorld, id: String) {
    world
        .send_as_current(Method::GET, &format!("/api/posts/{id}"), None)
        .await;
}

#[when(expr = "je modifie le post avec:")]
async fn update_last_post(world: &mut ApiWorld, step: &Step) {
    let id = world.last_id();
    world
        .send_as_current(
            Method::PUT,
            &format!("/api/posts/{id}"),
            Some(table_json(step)),
        )
        .await;
}

#[when(expr = "je modifie le post {string} avec:")]
async fn update_post_by_id(world: &mut ApiWorld, id: String, step: &Step) {
    world
        .send_as_current(
            Method::PUT,
            &format!("/api/posts/{id}"),
            Some(table_json(step)),
        )
        .await;
}

#[when("je supprime le post")]
async fn delete_last_post(world: &mut ApiWorld) {
    let id = world.last_id();
    world
        .send_as_current(Method::DELETE, &format!("/api/posts/{id}"), None)
        .await;
}

/// Éléments d'une réponse paginée (`data`).
fn page_items(world: &ApiWorld) -> &Vec<Value> {
    world.body["data"]
        .as_array()
        .unwrap_or_else(|| panic!("réponse paginée attendue : {}", world.body))
}

#[then("la réponse est une liste vide")]
async fn empty_list(world: &mut ApiWorld) {
    assert!(page_items(world).is_empty(), "{}", world.body);
}

#[then(expr = "la réponse contient exactement {int} posts")]
async fn posts_count(world: &mut ApiWorld, expected: usize) {
    assert_eq!(page_items(world).len(), expected, "{}", world.body);
}

#[then(expr = "aucun post de la liste n'a le titre {string}")]
async fn no_post_titled(world: &mut ApiWorld, title: String) {
    assert!(
        page_items(world).iter().all(|post| post["title"] != title),
        "un post \"{title}\" est visible : {}",
        world.body
    );
}

#[then("le post n'est plus accessible")]
async fn post_not_accessible(world: &mut ApiWorld) {
    let id = world.last_id();
    world
        .send_as_current(Method::GET, &format!("/api/posts/{id}"), None)
        .await;
    assert_eq!(world.status, 404, "{}", world.body);
}
