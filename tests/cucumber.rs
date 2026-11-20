//! Tests e2e Cucumber : `tests/features/*.feature`, exécutés sur le router complet.
//!
//! Chaque scénario commence par « la base de données est réinitialisée » (Background),
//! d'où l'exécution séquentielle.

mod common;

mod steps {
    pub mod auth_steps;
    pub mod common_steps;
    pub mod post_steps;
    pub mod user_steps;
    pub mod world;
}

use cucumber::World;
use steps::world::ApiWorld;

#[tokio::main]
async fn main() {
    ApiWorld::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features")
        .await;
}
