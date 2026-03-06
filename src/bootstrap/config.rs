use crate::bootstrap::models::{DatabaseConfig, JwtConfig, ServerConfig};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        let server = ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string()) // 8080 = port serveur, pas postgres
                .parse::<u16>()
                .unwrap_or(8080),
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        };

        let database = DatabaseConfig {
            url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string()) // bb8 : valeur raisonnable par défaut
                .parse::<u32>()
                .unwrap_or(10),
            acquire_timeout: env::var("DATABASE_ACQUIRE_TIMEOUT")
                .unwrap_or_else(|_| "30".to_string())
                .parse::<u64>()
                .unwrap_or(30),
            idle_timeout: env::var("DATABASE_IDLE_TIMEOUT")
                .unwrap_or_else(|_| "600".to_string())
                .parse::<u64>()
                .unwrap_or(600),
            max_lifetime: env::var("DATABASE_MAX_LIFETIME")
                .unwrap_or_else(|_| "1800".to_string())
                .parse::<u64>()
                .unwrap_or(1800),
        };

        let jwt = JwtConfig {
            secret: env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
            expiration: env::var("JWT_EXPIRATION")
                .unwrap_or_else(|_| "86400".to_string())
                .parse::<u32>()
                .unwrap_or(86400),
            refresh_secret: env::var("JWT_REFRESH_SECRET").expect("JWT_REFRESH_SECRET must be set"),
            refresh_expiration: env::var("JWT_REFRESH_EXPIRATION")
                .unwrap_or_else(|_| "604800".to_string())
                .parse::<u32>()
                .unwrap_or(604800),
        };

        Ok(Config {
            server,
            database,
            jwt,
        })
    }
}
