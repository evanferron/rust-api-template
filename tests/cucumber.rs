mod steps {
    pub mod auth_steps;
    pub mod post_steps;
    pub mod user_steps;
}
mod common;

use cucumber::World;
// use steps::auth_steps::AuthWorld;
// use steps::post_steps::PostsWorld;
use steps::user_steps::UsersWorld;

use crate::{
    common::reset_db,
    steps::{auth_steps::AuthWorld, post_steps::PostsWorld},
};

#[tokio::main]
async fn main() {
    // AUTH
    reset_db();
    AuthWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run("tests/features/auth.feature")
        .await;

    // POSTS
    reset_db();
    PostsWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run("tests/features/post.feature")
        .await;

    // USERS
    reset_db();
    UsersWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run("tests/features/user.feature")
        .await;
}
