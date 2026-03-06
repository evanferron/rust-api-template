use app::bootstrap;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let config = bootstrap::config::Config::from_env().expect("Failed to load config");
    bootstrap::server::Server::new(config).run().await
}
