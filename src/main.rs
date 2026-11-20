use api::app::server;
use api::config::Config;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    server::run(config).await
}
