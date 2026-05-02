use app::{config::api_config::Config, server::Server};
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let config = Config::from_env().expect("Failed to load config");
    Server::new(config).run().await
}
