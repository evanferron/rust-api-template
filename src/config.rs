//! Configuration de l'application, chargée depuis les variables d'environnement.
//!
//! Toutes les variables sont documentées dans `.env.example`.
//! Seules `DATABASE_URL`, `JWT_SECRET` et `JWT_REFRESH_SECRET` sont obligatoires.

use std::env;
use std::fmt;
use std::num::{NonZeroU32, NonZeroU64};
use std::str::FromStr;
use std::time::Duration;

use axum::http::HeaderValue;

#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub http: HttpConfig,
    pub rate_limit: RateLimitConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub environment: Environment,
    /// Délai maximal pour terminer les requêtes en cours à l'arrêt (`SIGTERM`, Ctrl+C).
    pub shutdown_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct HttpConfig {
    /// Origines autorisées par CORS.
    pub cors_allowed_origins: Vec<HeaderValue>,
    /// Durée maximale de traitement d'une requête.
    pub request_timeout: Duration,
    /// Taille maximale d'un corps de requête, en octets.
    pub body_limit: usize,
    /// Derrière un reverse proxy : l'IP du client est lue dans `X-Forwarded-For`.
    pub trust_proxy: bool,
}

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requêtes/minute sur toute l'API, par utilisateur (ou par IP si anonyme).
    pub requests_per_minute: NonZeroU32,
    /// Requêtes/minute par IP sur login / register / refresh.
    pub auth_requests_per_minute: NonZeroU32,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    /// Taille maximale du pool.
    pub max_connections: NonZeroU32,
    /// Attente maximale d'une connexion du pool.
    pub acquire_timeout: Duration,
    /// Durée après laquelle une connexion inutilisée est fermée.
    pub idle_timeout: Duration,
    /// Durée de vie maximale d'une connexion.
    pub max_lifetime: Duration,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    /// Durée de vie de l'access token, en secondes.
    pub expiration: u32,
    pub refresh_secret: String,
    /// Durée de vie du refresh token, en secondes.
    pub refresh_expiration: u32,
}

/// Environnement d'exécution (`ENVIRONMENT=development|production`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Logs lisibles, cookies sans `Secure` (HTTP local).
    Development,
    /// Logs JSON, cookies `Secure`.
    Production,
}

impl Environment {
    pub fn is_production(self) -> bool {
        self == Environment::Production
    }
}

impl FromStr for Environment {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "production" | "prod" => Ok(Environment::Production),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Environment::Development => "development",
            Environment::Production => "production",
        })
    }
}

/// Erreurs de configuration : liste **toutes** les variables manquantes ou invalides.
#[derive(Debug)]
pub struct ConfigError(pub Vec<String>);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "invalid configuration:")?;
        for problem in &self.0 {
            writeln!(f, "  - {problem}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Charge la configuration depuis les variables d'environnement.
    ///
    /// Une valeur présente mais invalide (ex: `SERVER_PORT=abc`) est une erreur,
    /// jamais remplacée silencieusement par la valeur par défaut.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_source(|key| env::var(key).ok())
    }

    /// Variante testable : `get` fournit la valeur d'une variable.
    pub fn from_source(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let mut env = EnvReader {
            get,
            errors: Vec::new(),
        };

        let config = Config {
            server: ServerConfig {
                host: env.or("SERVER_HOST", "127.0.0.1".to_string()),
                port: env.or("SERVER_PORT", 8080),
                environment: env.or("ENVIRONMENT", Environment::Development),
                shutdown_timeout: env.seconds("SHUTDOWN_TIMEOUT_SECS", 30),
            },
            http: HttpConfig {
                cors_allowed_origins: env.origins("CORS_ALLOWED_ORIGINS", "http://localhost:5173"),
                request_timeout: env.seconds("REQUEST_TIMEOUT_SECS", 30),
                body_limit: env.or("BODY_LIMIT_BYTES", 1024 * 1024),
                trust_proxy: env.or("TRUST_PROXY", false),
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: env.or("RATE_LIMIT_RPM", NonZeroU32::new(120).unwrap()),
                auth_requests_per_minute: env
                    .or("RATE_LIMIT_AUTH_RPM", NonZeroU32::new(10).unwrap()),
            },
            database: DatabaseConfig {
                url: env.required("DATABASE_URL"),
                max_connections: env.or("DATABASE_MAX_CONNECTIONS", NonZeroU32::new(10).unwrap()),
                acquire_timeout: env.seconds("DATABASE_ACQUIRE_TIMEOUT_SECS", 30),
                idle_timeout: env.seconds("DATABASE_IDLE_TIMEOUT_SECS", 600),
                max_lifetime: env.seconds("DATABASE_MAX_LIFETIME_SECS", 1800),
            },
            jwt: JwtConfig {
                secret: env.required("JWT_SECRET"),
                expiration: env.or("JWT_EXPIRATION_SECS", 86400),
                refresh_secret: env.required("JWT_REFRESH_SECRET"),
                refresh_expiration: env.or("JWT_REFRESH_EXPIRATION_SECS", 604800),
            },
        };

        // Une variable renommée est une erreur explicite plutôt qu'une valeur ignorée en silence
        for (old, new) in RENAMED_VARS {
            if (env.get)(old).is_some() {
                env.errors.push(format!("{old} has been renamed to {new}"));
            }
        }

        if env.errors.is_empty() {
            Ok(config)
        } else {
            Err(ConfigError(env.errors))
        }
    }
}

/// Anciennes variables → nouveau nom.
const RENAMED_VARS: [(&str, &str); 6] = [
    ("FRONTEND_URL", "CORS_ALLOWED_ORIGINS"),
    ("DATABASE_ACQUIRE_TIMEOUT", "DATABASE_ACQUIRE_TIMEOUT_SECS"),
    ("DATABASE_IDLE_TIMEOUT", "DATABASE_IDLE_TIMEOUT_SECS"),
    ("DATABASE_MAX_LIFETIME", "DATABASE_MAX_LIFETIME_SECS"),
    ("JWT_EXPIRATION", "JWT_EXPIRATION_SECS"),
    ("JWT_REFRESH_EXPIRATION", "JWT_REFRESH_EXPIRATION_SECS"),
];

/// Lit les variables en accumulant les erreurs au lieu de s'arrêter à la première.
struct EnvReader<F> {
    get: F,
    errors: Vec<String>,
}

impl<F: Fn(&str) -> Option<String>> EnvReader<F> {
    /// Variable obligatoire et non vide.
    fn required(&mut self, key: &str) -> String {
        match (self.get)(key).filter(|v| !v.trim().is_empty()) {
            Some(value) => value,
            None => {
                self.errors.push(format!("{key} is required"));
                String::new()
            }
        }
    }

    /// Variable optionnelle : `default` si absente, erreur si invalide.
    fn or<T: FromStr>(&mut self, key: &str, default: T) -> T {
        match (self.get)(key) {
            None => default,
            Some(raw) => raw.trim().parse().unwrap_or_else(|_| {
                self.invalid(key, &raw);
                default
            }),
        }
    }

    /// Durée en secondes, strictement positive.
    fn seconds(&mut self, key: &str, default: u64) -> Duration {
        let default = NonZeroU64::new(default).expect("default duration must be > 0");
        Duration::from_secs(self.or(key, default).get())
    }

    /// Liste d'origines séparées par des virgules, chacune valide comme en-tête HTTP.
    fn origins(&mut self, key: &str, default: &str) -> Vec<HeaderValue> {
        let raw = (self.get)(key).unwrap_or_else(|| default.to_string());
        let origins: Vec<HeaderValue> = raw
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .filter_map(|origin| match HeaderValue::from_str(origin) {
                Ok(value) => Some(value),
                Err(_) => {
                    self.invalid(key, origin);
                    None
                }
            })
            .collect();
        if origins.is_empty() {
            self.errors
                .push(format!("{key} must contain at least one origin"));
        }
        origins
    }

    fn invalid(&mut self, key: &str, raw: &str) {
        self.errors
            .push(format!("{key} has an invalid value: {raw:?}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn load(vars: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_source(|key| vars.get(key).cloned())
    }

    const REQUIRED: [(&str, &str); 3] = [
        ("DATABASE_URL", "postgres://localhost/db"),
        ("JWT_SECRET", "secret"),
        ("JWT_REFRESH_SECRET", "refresh"),
    ];

    fn with(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
        let mut vars = REQUIRED.to_vec();
        vars.extend_from_slice(extra);
        load(&vars)
    }

    #[test]
    fn defaults_apply_when_optional_vars_are_missing() {
        let config = load(&REQUIRED).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.environment, Environment::Development);
        assert_eq!(config.server.shutdown_timeout, Duration::from_secs(30));
        assert_eq!(config.http.cors_allowed_origins, ["http://localhost:5173"]);
        assert!(!config.http.trust_proxy);
        assert_eq!(config.rate_limit.requests_per_minute.get(), 120);
        assert_eq!(config.database.max_connections.get(), 10);
    }

    #[test]
    fn all_problems_are_reported_at_once() {
        let err = load(&[("SERVER_PORT", "abc"), ("JWT_SECRET", " ")]).unwrap_err();
        let problems = err.0.join("\n");
        assert!(problems.contains("SERVER_PORT has an invalid value"));
        assert!(problems.contains("DATABASE_URL is required"));
        assert!(problems.contains("JWT_SECRET is required"));
        assert!(problems.contains("JWT_REFRESH_SECRET is required"));
        assert_eq!(err.0.len(), 4);
    }

    #[test]
    fn invalid_value_is_never_silently_replaced() {
        assert!(with(&[("DATABASE_MAX_CONNECTIONS", "-1")]).is_err());
        assert!(with(&[("DATABASE_MAX_CONNECTIONS", "0")]).is_err());
        assert!(with(&[("RATE_LIMIT_RPM", "0")]).is_err());
        assert!(with(&[("REQUEST_TIMEOUT_SECS", "0")]).is_err());
        assert!(with(&[("TRUST_PROXY", "yes")]).is_err());
        assert!(with(&[("ENVIRONMENT", "staging")]).is_err());
    }

    #[test]
    fn renamed_variables_are_reported() {
        let err = with(&[("JWT_EXPIRATION", "3600")]).unwrap_err();
        assert_eq!(
            err.0,
            ["JWT_EXPIRATION has been renamed to JWT_EXPIRATION_SECS"]
        );
    }

    #[test]
    fn environment_accepts_short_names() {
        let config = with(&[("ENVIRONMENT", "PROD")]).unwrap();
        assert!(config.server.environment.is_production());
    }

    #[test]
    fn cors_origins_are_a_comma_separated_list() {
        let config = with(&[(
            "CORS_ALLOWED_ORIGINS",
            "https://app.example.com, http://localhost:5173",
        )])
        .unwrap();
        assert_eq!(
            config.http.cors_allowed_origins,
            ["https://app.example.com", "http://localhost:5173"]
        );
        assert!(with(&[("CORS_ALLOWED_ORIGINS", " , ")]).is_err());
    }
}
