use app::{infra::config::Config, launch::Server};
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let config = Config::from_env();
    Server::new(config).run().await
}
