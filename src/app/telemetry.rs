use tracing_subscriber::{
    EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// Initialise le système de logs selon l'environnement.
///
/// - `development` → texte coloré et lisible dans le terminal
/// - `production`  → JSON structuré, une ligne par événement
///
/// Le niveau de log est contrôlable via la variable d'environnement `RUST_LOG`.
/// Valeur par défaut : `info` pour l'app, `debug` pour tower_http (logs HTTP).
pub fn init(environment: &str) {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Par défaut : logs HTTP visibles, SQL masqué sauf si explicitement demandé
        EnvFilter::new("info,tower_http::trace::on_response=info,tower_http=off")
    });

    match environment {
        "production" => init_json(env_filter),
        _ => init_pretty(env_filter),
    }
}

/// Format pretty — coloré, lisible, pour le développement.
fn init_pretty(env_filter: EnvFilter) {
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .pretty()
                .with_target(true) // affiche le module source (ex: tower_http::trace)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false),
        )
        .init();
}

/// Format JSON — structuré, une ligne par événement, pour la production.
fn init_json(env_filter: EnvFilter) {
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .json()
                .with_target(true)
                .with_current_span(true) // inclut le contexte du span parent (ex: request_id)
                .with_span_list(false) // évite la verbosité excessive
                .with_span_events(FmtSpan::CLOSE),
        )
        .init();
}
