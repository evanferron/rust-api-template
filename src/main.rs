use api::app::server;
use api::config::Config;
use dotenvy::dotenv;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    dotenv().ok();
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return std::process::ExitCode::FAILURE;
        }
    };

    match server::run(config).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            // Les logs sont déjà initialisés par server::run
            tracing::error!(error = %err, "Server failed");
            std::process::ExitCode::FAILURE
        }
    }
}
