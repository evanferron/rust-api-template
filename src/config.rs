use std::env;
use std::fmt;
use std::str::FromStr;

/// Configuration de l'application, chargée depuis l'environnement (voir `.env.example`).
#[derive(Debug, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub environment: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub acquire_timeout: u64,
    pub idle_timeout: u64,
    pub max_lifetime: u64,
}

#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration: u32,
    pub refresh_secret: String,
    pub refresh_expiration: u32,
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
    /// Obligatoires : `DATABASE_URL`, `JWT_SECRET`, `JWT_REFRESH_SECRET`.
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
                environment: env.or("ENVIRONMENT", "development".to_string()),
            },
            database: DatabaseConfig {
                url: env.required("DATABASE_URL"),
                max_connections: env.or("DATABASE_MAX_CONNECTIONS", 10),
                acquire_timeout: env.or("DATABASE_ACQUIRE_TIMEOUT", 30),
                idle_timeout: env.or("DATABASE_IDLE_TIMEOUT", 600),
                max_lifetime: env.or("DATABASE_MAX_LIFETIME", 1800),
            },
            jwt: JwtConfig {
                secret: env.required("JWT_SECRET"),
                expiration: env.or("JWT_EXPIRATION", 86400),
                refresh_secret: env.required("JWT_REFRESH_SECRET"),
                refresh_expiration: env.or("JWT_REFRESH_EXPIRATION", 604800),
            },
        };

        if env.errors.is_empty() {
            Ok(config)
        } else {
            Err(ConfigError(env.errors))
        }
    }
}

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
                self.errors
                    .push(format!("{key} has an invalid value: {raw:?}"));
                default
            }),
        }
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

    #[test]
    fn defaults_apply_when_optional_vars_are_missing() {
        let config = load(&REQUIRED).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.max_connections, 10);
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
        let mut vars = REQUIRED.to_vec();
        vars.push(("DATABASE_MAX_CONNECTIONS", "-1"));
        assert!(load(&vars).is_err());
    }
}
