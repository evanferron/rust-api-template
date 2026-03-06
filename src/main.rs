use app::app;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let config = app::config::Config::from_env().expect("Failed to load config");
    app::server::Server::new(config).run().await
}
