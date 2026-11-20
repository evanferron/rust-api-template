//! Steps communs à toutes les features : réinitialisation, connexion, assertions.

use cucumber::gherkin::Step;
use cucumber::{given, then, when};

use super::world::{ApiWorld, table};
use crate::common::reset_db;

#[given("la base de données est réinitialisée")]
async fn reset_database(_world: &mut ApiWorld) {
    reset_db().await;
}

/// Inscrit l'utilisateur si besoin, le connecte et en fait l'utilisateur courant.
#[given(expr = "je suis connecté en tant que {string} avec le mot de passe {string}")]
#[when(expr = "je suis connecté en tant que {string} avec le mot de passe {string}")]
async fn logged_in_as(world: &mut ApiWorld, email: String, password: String) {
    world.login_as(&email, &password).await;
}

#[then(expr = "le statut de la réponse est {int}")]
async fn status_is(world: &mut ApiWorld, expected: u16) {
    assert_eq!(world.status, expected, "réponse : {}", world.body);
}

/// Compare des champs texte de la réponse, ex : `| code | VALIDATION_FAILED |`.
#[then(expr = "la réponse contient:")]
async fn response_contains(world: &mut ApiWorld, step: &Step) {
    for (key, expected) in table(step) {
        let actual = world.body[&key].as_str().unwrap_or_default();
        assert_eq!(actual, expected, "body[\"{key}\"] dans {}", world.body);
    }
}

#[then(expr = "la réponse contient un champ {string}")]
async fn response_has_field(world: &mut ApiWorld, field: String) {
    assert!(
        world.body.get(&field).is_some(),
        "champ \"{field}\" absent de {}",
        world.body
    );
}

/// Compare la valeur JSON brute (`true`, `42`, `"texte"`).
#[then(expr = "le champ {string} de la réponse vaut {string}")]
async fn field_equals(world: &mut ApiWorld, field: String, expected: String) {
    assert_eq!(
        world.body[&field].to_string(),
        expected,
        "body[\"{field}\"]"
    );
}

/// Erreur de validation (`VALIDATION_FAILED`) sur un champ précis.
#[then(expr = "la réponse signale une erreur sur le champ {string}")]
async fn field_error(world: &mut ApiWorld, field: String) {
    let has_errors = world.body["errors"][&field]
        .as_array()
        .is_some_and(|messages| !messages.is_empty());
    assert!(
        has_errors,
        "aucune erreur pour \"{field}\" dans {}",
        world.body
    );
}

#[then("le mot de passe n'apparaît pas dans la réponse")]
#[then("le hash du mot de passe n'apparaît pas dans la réponse")]
async fn no_password(world: &mut ApiWorld) {
    assert!(world.body.get("password").is_none(), "{}", world.body);
    assert!(world.body.get("password_hash").is_none(), "{}", world.body);
}
