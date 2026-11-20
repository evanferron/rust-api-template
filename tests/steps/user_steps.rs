//! Steps de `user.feature` (et suppression de compte, utilisée aussi par `post.feature`).

use axum::http::Method;
use cucumber::gherkin::Step;
use cucumber::{then, when};

use super::world::{ApiWorld, table_json};

#[when("je liste les utilisateurs")]
async fn list_users(world: &mut ApiWorld) {
    world.send_as_current(Method::GET, "/api/users", None).await;
}

#[when("je liste les utilisateurs sans token")]
async fn list_users_without_token(world: &mut ApiWorld) {
    world.send(Method::GET, "/api/users", None, None).await;
}

#[when("je récupère mon profil par son id")]
async fn get_own_profile(world: &mut ApiWorld) {
    let id = world.user_id_of(&world.current_user()).await;
    world
        .send_as_current(Method::GET, &format!("/api/users/{id}"), None)
        .await;
}

#[when(expr = "je récupère l'utilisateur avec l'id {string}")]
async fn get_user_by_id(world: &mut ApiWorld, id: String) {
    world
        .send_as_current(Method::GET, &format!("/api/users/{id}"), None)
        .await;
}

#[when(expr = "je modifie mon profil avec:")]
async fn update_own_profile(world: &mut ApiWorld, step: &Step) {
    let id = world.user_id_of(&world.current_user()).await;
    world
        .send_as_current(
            Method::PUT,
            &format!("/api/users/{id}"),
            Some(table_json(step)),
        )
        .await;
}

/// L'utilisateur courant tente de modifier le profil d'un autre.
#[when(expr = "je modifie le profil de {string} avec:")]
async fn update_other_profile(world: &mut ApiWorld, email: String, step: &Step) {
    let id = world.user_id_of(&email).await;
    world
        .send_as_current(
            Method::PUT,
            &format!("/api/users/{id}"),
            Some(table_json(step)),
        )
        .await;
}

#[when("je supprime mon compte")]
async fn delete_own_account(world: &mut ApiWorld) {
    let id = world.user_id_of(&world.current_user()).await;
    world
        .send_as_current(Method::DELETE, &format!("/api/users/{id}"), None)
        .await;
}

/// L'utilisateur courant tente de supprimer le compte d'un autre.
#[when(expr = "je supprime le compte de {string}")]
async fn delete_other_account(world: &mut ApiWorld, email: String) {
    let id = world.user_id_of(&email).await;
    world
        .send_as_current(Method::DELETE, &format!("/api/users/{id}"), None)
        .await;
}

#[then(expr = "la réponse est une liste de {int} utilisateur")]
async fn users_count(world: &mut ApiWorld, expected: usize) {
    let users = world.body["data"]
        .as_array()
        .unwrap_or_else(|| panic!("réponse paginée attendue : {}", world.body));
    assert_eq!(users.len(), expected, "{}", world.body);
}
