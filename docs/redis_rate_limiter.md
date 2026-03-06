# Migration du rate limiting vers Redis

## Pourquoi migrer vers Redis ?

L'implémentation actuelle stocke les compteurs en mémoire (`DashMap`). C'est suffisant pour une instance unique, mais pose deux problèmes en production :

- **Restart** — les compteurs sont remis à zéro à chaque redémarrage du serveur
- **Multi-instances** — si tu déploies plusieurs instances (load balancer), chaque instance a ses propres compteurs. Un attaquant peut contourner le rate limit en alternant les instances.

Redis centralise les compteurs, les rend persistants et les partage entre toutes les instances.

---

## Dépendances à ajouter

```toml
# Cargo.toml
redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
```

Retire `governor` et `dashmap` si tu n'en as plus besoin ailleurs.

---

## Variable d'environnement

Dans `.env` :

```env
REDIS_URL=redis://localhost:6379
```

Dans `app/models.rs`, ajoute `redis_url` à `AppConfig` :

```env
# Format avec auth si Redis est sécurisé
REDIS_URL=redis://:password@localhost:6379
```

---

## 1. Mettre à jour `AppState`

```rust
// app/models.rs
use redis::aio::ConnectionManager;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<AsyncPgConnection>,
    pub config: Config,
    pub services: Services,
    pub repositories: Arc<Repositories>,
    pub redis: ConnectionManager,  // ← remplace RateLimitStore
}
```

---

## 2. Initialiser la connexion Redis dans `server.rs`

```rust
// app/server.rs
use redis::Client;

// Après la création du pool Diesel, avant l'AppState
let redis_client = Client::open(config.database.redis_url.as_str())
    .expect("Invalid Redis URL");

let redis = redis_client
    .get_connection_manager()
    .await
    .expect("Failed to connect to Redis");

tracing::info!("Redis connection established");

let app_state = AppState {
    pool,
    config: config.clone(),
    services,
    repositories,
    redis,
};
```

---

## 3. Réécrire `core/middlewares/rate_limit.rs`

L'algorithme utilisé est **sliding window** avec une clé Redis par client.
Chaque requête incrémente un compteur avec un TTL — si le compteur dépasse la limite, on rejette.

```rust
// core/middlewares/rate_limit.rs
use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;
use std::net::SocketAddr;
use std::time::Duration;

use crate::app::models::AppState;
use crate::core::errors::ApiError;
use crate::modules::auth::helpers::Claims;

// ---------------------------------------------------------------------------
// Helper interne
// ---------------------------------------------------------------------------

/// Vérifie et incrémente le compteur pour une clé donnée.
/// Retourne une erreur si la limite est dépassée.
///
/// Algorithme : fixed window avec TTL Redis.
/// - Clé : `rate_limit:{prefix}:{client_id}`
/// - TTL : réinitialisé à chaque nouvelle fenêtre
async fn check_rate_limit(
    redis: &mut redis::aio::ConnectionManager,
    key: &str,
    max_requests: u64,
    window_secs: u64,
) -> Result<(), ()> {
    // INCR est atomique — pas de race condition
    let count: u64 = redis.incr(key, 1).await.unwrap_or(0);

    if count == 1 {
        // Première requête de la fenêtre — on pose le TTL
        let _: () = redis
            .expire(key, window_secs as i64)
            .await
            .unwrap_or(());
    }

    if count > max_requests {
        return Err(());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Middleware — routes publiques (par IP)
// ---------------------------------------------------------------------------

pub async fn rate_limit_by_ip(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let ip = addr.ip().to_string();
    let key = format!("rate_limit:ip:{}", ip);

    let mut redis = state.redis.clone();

    check_rate_limit(&mut redis, &key, 10, 60)
        .await
        .map_err(|_| {
            tracing::warn!(ip = %ip, "Rate limit exceeded (IP)");
            ApiError::RateLimitExceeded {
                client_id: ip,
                max_requests: 10,
                window_duration: Duration::from_secs(60),
            }
        })?;

    Ok(next.run(req).await)
}

// ---------------------------------------------------------------------------
// Middleware — routes protégées (par user)
// ---------------------------------------------------------------------------

pub async fn rate_limit_by_user(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let user_id = req
        .extensions()
        .get::<Claims>()
        .map(|c| c.sub.to_string())
        .ok_or_else(|| ApiError::Authentication("Missing claims".to_string()))?;

    let key = format!("rate_limit:user:{}", user_id);
    let mut redis = state.redis.clone();

    check_rate_limit(&mut redis, &key, 120, 60)
        .await
        .map_err(|_| {
            tracing::warn!(user_id = %user_id, "Rate limit exceeded (user)");
            ApiError::RateLimitExceeded {
                client_id: user_id,
                max_requests: 120,
                window_duration: Duration::from_secs(60),
            }
        })?;

    Ok(next.run(req).await)
}
```

---

## 4. Mettre à jour la config

```rust
// app/config.rs — ajouter redis_url à DatabaseConfig ou créer une RedisConfig
#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

// Dans Config
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub redis: RedisConfig,    // ← ajouter
}

// Dans from_env()
let redis = RedisConfig {
    url: env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
};
```

---

## 5. Docker Compose

Pour lancer Redis en local :

```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: app_db
    ports:
      - "5432:5432"

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --save 60 1  # persistence toutes les 60s si 1 changement
```

---

## Comparaison des deux approches

| | Mémoire (actuel) | Redis |
|---|---|---|
| **Setup** | ✅ Aucun | ⚠️ Service supplémentaire |
| **Restart** | ❌ Compteurs perdus | ✅ Persistant |
| **Multi-instances** | ❌ Compteurs isolés | ✅ Partagés |
| **Performance** | ✅ ~0ms | ⚠️ ~1-2ms (réseau local) |
| **Fuite mémoire** | ⚠️ `retain_recent()` requis | ✅ TTL natif |
| **Adapté pour** | Dev / instance unique | Production / scaling |

---

## Notes

- L'algorithme **fixed window** utilisé ici est simple mais peut laisser passer 2x la limite en cas de burst en fin/début de fenêtre. Pour une protection plus stricte, utilise l'algorithme **sliding window** avec des structures Redis de type `ZSET`.
- `ConnectionManager` gère automatiquement la reconnexion en cas de déconnexion Redis — pas besoin de gérer les erreurs de connexion manuellement.
- Les clés Redis ont un TTL natif — pas besoin de nettoyage périodique contrairement à la version mémoire.
