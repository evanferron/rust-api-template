# Contexte complet — rust-api-template

---

## 1. Vue d'ensemble

Template API REST production-ready en Rust. Objectif : fournir une base solide et réutilisable qui accélère le développement de nouvelles APIs.

**Principes directeurs :**

- Types concrets partout — pas de `Box<dyn Trait>` pour les repositories internes
- Fonctions libres dans les services — pas de struct `XxxService`
- Repositories = structs vides avec méthodes statiques async
- AppState minimal — pool + config + rate_limit uniquement
- Macros granulaires à la carte — chaque repository choisit ce dont il a besoin

---

## 2. Stack technique complète

| Composant | Technologie | Version | Notes |
| --------- | ----------- | ------- | ----- |
| Framework | Axum | 0.8 | `{id}` pas `:id` pour les params de route |
| ORM | Diesel | 2.3.6 | features: postgres, uuid, chrono |
| ORM async | diesel-async | 0.7.4 | features: postgres, bb8 |
| Pool | bb8 | — | async, via diesel-async |
| DB | PostgreSQL | 16 | uuid-ossp activé via migration initiale |
| Auth | jsonwebtoken | 10.3.0 | feature `rust_crypto`, HS256 uniquement |
| Hash | bcrypt | 0.19.0 | cost 4 en dev, 12 en prod |
| Validation | validator | 0.19 | feature `derive` |
| Logs | tracing + tracing-subscriber | 0.3 | pretty dev, JSON prod, feature `env-filter json` |
| Rate limiting | governor + dashmap | 0.8 + 6 | en mémoire, par IP et par user |
| Doc API | utoipa + swagger-ui | 5.4 + 9.0.2 | features: `axum_extras`, `uuid`, `chrono` |
| HTTP extras | axum-extra | 0.10 | feature `cookie` pour refresh token |
| Cookies | time | 0.3 | requis par axum-extra cookie |
| Erreurs | thiserror | 2 | — |
| Erreurs génériques | anyhow | 1 | — |
| Sérialisation | serde + serde_json | 1 | feature `derive` |
| UUID | uuid | 1 | features: v4, serde |
| Dates | chrono | 0.4 | feature `serde` |
| Env | dotenvy | 0.15 | — |
| Compression | tower-http | 0.6.8 | features: cors, trace, compression-full, limit |
| Edition Rust | 2024 | — | Rust 1.85+ requis |

**Cargo.toml — binaires déclarés :**

```toml
[lib]
name = "app"
path = "src/lib.rs"

[[bin]]
name = "server"
path = "src/main.rs"

[[bin]]
name = "generate"
path = "src/bin/generate.rs"
```

**Profil release optimisé :**

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

---

## 3. Structure complète du projet

```tree
rust-api-template/
├── .cargo/
│   └── config.toml          ← test-threads = 1 (obligatoire pour les e2e)
├── .github/
│   ├── copilot-instructions.md
│   └── workflows/
│       ├── build.yml        ← CI : lint + tests + SonarCloud + cargo audit
│       └── cd.yml           ← CD : Docker Hub + Render deploy hook
├── migrations/
│   ├── 00000000000000_diesel_initial_setup/
│   ├── 2026-03-06-..._create_users/
│   └── 2026-03-07-..._create_posts/
├── src/
│   ├── core/
│   │   ├── errors.rs        ← ApiError central
│   │   ├── logger.rs        ← init(env: &str)
│   │   ├── repository.rs    ← BaseRepository trait + toutes les macros
│   │   ├── validator.rs     ← ValidatedJson<T> extractor
│   │   └── middlewares/
│   │       ├── auth.rs      ← require_auth
│   │       ├── rate_limit.rs ← rate_limit_by_ip + rate_limit_by_user
│   │       └── mod.rs
│   ├── db/
│   │   ├── schema.rs        ← généré par `diesel print-schema`
│   │   ├── user/
│   │   │   ├── model.rs     ← User + NewUser
│   │   │   ├── repository.rs ← UserRepository
│   │   │   └── mod.rs
│   │   └── post/
│   │       ├── model.rs     ← Post + NewPost
│   │       ├── repository.rs ← PostRepository
│   │       └── mod.rs
│   ├── config/
│   │   ├── config.rs        ← Config::from_env()
│   │   ├── state.rs         ← AppState + RateLimitStore
│   │   └── mod.rs
│   ├── server/
│   │   ├── router.rs        ← create_router(state) → Router
│   │   ├── swagger.rs       ← ApiDoc (utoipa)
│   │   └── mod.rs
│   ├── modules/
│   │   ├── auth/
│   │   │   ├── dto.rs       ← LoginRequest, RegisterRequest, TokenResponse...
│   │   │   ├── handler.rs   ← login, register, refresh, logout
│   │   │   ├── helpers.rs   ← create_token, verify_token, hash_password, verify_password
│   │   │   ├── service.rs   ← login(), register(), refresh()
│   │   │   └── mod.rs
│   │   ├── health/
│   │   │   ├── dto.rs       ← HealthResponse
│   │   │   ├── handler.rs   ← health_check
│   │   │   └── mod.rs
│   │   ├── user/
│   │   │   ├── dto.rs       ← UserResponse, UpdateUserRequest
│   │   │   ├── handler.rs   ← get_me, update_me, delete_me, get_all (admin)
│   │   │   ├── service.rs
│   │   │   └── mod.rs
│   │   ├── post/
│   │   │   ├── dto.rs       ← PostResponse, CreatePostRequest, UpdatePostRequest
│   │   │   ├── handler.rs   ← CRUD + get_by_user
│   │   │   ├── service.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   ├── bin/
│   │   └── generate.rs      ← CLI scaffold
│   ├── lib.rs               ← pub use + re-exports
│   └── main.rs              ← point d'entrée
├── tests/
│   ├── common/
│   │   └── mod.rs           ← get_test_app(), reset_db(), seed_user_and_login()
│   ├── cucumber.rs          ← orchestrateur des features Cucumber
│   ├── features/
│   │   ├── auth.feature
│   │   ├── post.feature
│   │   └── user.feature
│   └── steps/
│       ├── auth_steps.rs
│       ├── post_steps.rs
│       └── user_steps.rs
├── docs/
│   ├── module_generator.md
│   ├── tests.md
│   └── rust_docs.md
├── .cursorrules
├── .dockerignore
├── .env
├── .env.example
├── .gitignore
├── audit.toml               ← RUSTSEC-2023-0071 ignoré (HS256 only)
├── Cargo.toml
├── Cargo.lock
├── diesel.toml
├── docker-compose.yml       ← postgres + postgres_test
├── Dockerfile               ← multi-stage avec cargo-chef
├── render.yaml              ← Blueprint Render.com
├── sonar-project.properties
└── README.md
```

---

## 4. AppState et Config

```rust
// config/state.rs
pub type RateLimitStore = Arc<DashMap<String, Arc<DefaultDirectRateLimiter>>>;

pub struct AppState {
    pub pool: Pool<AsyncPgConnection>,  // bb8
    pub config: Config,
    pub rate_limit: RateLimitStore,
}

// config/config.rs
pub struct Config {
    pub server_host: String,         // SERVER_HOST
    pub server_port: u16,            // SERVER_PORT
    pub environment: String,         // ENVIRONMENT
    pub database_url: String,        // DATABASE_URL
    pub jwt_secret: String,          // JWT_SECRET
    pub jwt_expiration: i64,         // JWT_EXPIRATION (secondes)
    pub jwt_refresh_secret: String,  // JWT_REFRESH_SECRET
    pub jwt_refresh_expiration: i64, // JWT_REFRESH_EXPIRATION (secondes)
    pub frontend_url: String,        // FRONTEND_URL
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> { ... }
}
```

---

## 5. Gestion des erreurs

```rust
// core/errors.rs
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded { retry_after: u64 },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg)        => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::Conflict(msg)        => (StatusCode::CONFLICT, msg.clone()),
            ApiError::Authentication(msg)  => (StatusCode::UNAUTHORIZED, msg.clone()),
            ApiError::Authorization(msg)   => (StatusCode::FORBIDDEN, msg.clone()),
            ApiError::Validation(msg)      => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            ApiError::Internal(msg)        => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ApiError::RateLimitExceeded { retry_after } => (StatusCode::TOO_MANY_REQUESTS, ...),
        };
        Json(json!({ "error": message })).into_response()
    }
}

// Conversion depuis diesel::result::Error
impl From<diesel::result::Error> for ApiError {
    fn from(e: diesel::result::Error) -> Self {
        match e {
            diesel::result::Error::NotFound => ApiError::NotFound("Resource not found".into()),
            diesel::result::Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) => ApiError::Conflict(...),
            _ => ApiError::Internal(e.to_string()),
        }
    }
}
```

---

## 6. Repository générique — macros

```rust
// core/repository.rs

// Trait de base
pub trait BaseRepository {
    type Model;
    type Id;
    async fn find_all(conn: &mut AsyncPgConnection) -> Result<Vec<Self::Model>, ApiError>;
    async fn find_by_id(conn: &mut AsyncPgConnection, id: Self::Id) -> Result<Option<Self::Model>, ApiError>;
    async fn delete(conn: &mut AsyncPgConnection, id: Self::Id) -> Result<usize, ApiError>;
}

// Macro composite — génère find_all + find_by_id + delete + impl BaseRepository
#[macro_export]
macro_rules! impl_base_repository {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => { ... }
}

// Macros granulaires
#[macro_export] macro_rules! impl_find_all { ... }       // SELECT * FROM table
#[macro_export] macro_rules! impl_find_by_id { ... }     // SELECT * WHERE id = ?
#[macro_export] macro_rules! impl_delete { ... }         // DELETE WHERE id = ?
#[macro_export] macro_rules! impl_exists { ... }         // SELECT EXISTS(...)
#[macro_export] macro_rules! impl_count { ... }          // SELECT COUNT(*)
#[macro_export] macro_rules! impl_find_paginated { ... } // LIMIT/OFFSET + total
#[macro_export] macro_rules! impl_soft_delete { ... }    // updated deleted_at + find_active + restore
```

**IMPORTANT — syntaxe correcte dans les macros Diesel :**

```rust
// ❌ NE PAS FAIRE — cause une erreur de parsing
use diesel::prelude::*;
$table::table.load::<$model>(conn)...

// ✅ CORRECT — utiliser un alias
use $table as _table;
_table::table.load::<$model>(conn)...
```

**Utilisation dans un repository :**

```rust
pub struct PostRepository;

// Macro composite (recommandée comme base)
crate::impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);

// Ajouter à la carte selon les besoins
crate::impl_exists!(PostRepository, crate::db::schema::posts, Uuid);
crate::impl_count!(PostRepository, crate::db::schema::posts);
crate::impl_find_paginated!(PostRepository, Post, crate::db::schema::posts, created_at);
// Soft delete — nécessite une colonne deleted_at TIMESTAMP NULL dans la table
crate::impl_soft_delete!(PostRepository, Post, crate::db::schema::posts, Uuid);

impl PostRepository {
    // Méthodes spécifiques au domaine uniquement
    pub async fn find_by_user_id(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
    ) -> Result<Vec<Post>, ApiError> {
        use crate::db::schema::posts::dsl;
        dsl::posts
            .filter(dsl::user_id.eq(user_id))
            .load::<Post>(conn)
            .await
            .map_err(ApiError::from)
    }
}
```

---

## 7. Modèles Diesel

### User

```rust
// db/user/model.rs
#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::db::schema::users)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password: String,  // colonne "password" (pas "password_hash")
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::users)]
pub struct NewUser {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub name: String,
}
```

### Post (One-to-Many avec User)

```rust
// db/post/model.rs
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations)]
#[diesel(table_name = crate::db::schema::posts)]
#[diesel(belongs_to(User))]       // clé étrangère user_id → users.id
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::db::schema::posts)]
pub struct NewPost {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
}
```

### schema.rs (relations)

```rust
// db/schema.rs — déclarations obligatoires pour les jointures
diesel::joinable!(posts -> users (user_id));
diesel::allow_tables_to_appear_in_same_query!(users, posts);
```

### Jointure avec auteur

```rust
// repository.rs — récupérer un post avec son auteur
pub async fn find_with_author(
    conn: &mut AsyncPgConnection,
    id: Uuid,
) -> Result<Option<(Post, User)>, ApiError> {
    use crate::db::schema::{posts, users};
    posts::table
        .inner_join(users::table)
        .filter(posts::id.eq(id))
        .select((Post::as_select(), User::as_select()))
        .first::<(Post, User)>(conn)
        .await
        .optional()
        .map_err(ApiError::from)
}
```

---

## 8. Pattern complet d'un module

### dto.rs

```rust
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;
use crate::db::post::model::Post;

// Response — sérialisé en JSON
#[derive(Debug, Serialize, ToSchema)]
pub struct PostResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Post> for PostResponse {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            user_id: post.user_id,
            title: post.title,
            content: post.content,
            created_at: post.created_at.to_string(),
            updated_at: post.updated_at.to_string(),
        }
    }
}

// Create request
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]   // Rejette les champs inconnus
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(min = 1))]
    pub content: String,
}

// Update request — Option<T> pour PATCH semantics
#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdatePostRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,
    #[validate(length(min = 1))]
    pub content: Option<String>,
}
```

### service.rs

```rust
use diesel_async::AsyncPgConnection;
use uuid::Uuid;
use crate::core::errors::ApiError;
use crate::db::post::repository::PostRepository;
use super::dto::{CreatePostRequest, PostResponse, UpdatePostRequest};

// IMPORTANT : fonctions libres, pas de struct Service

pub async fn get_all(conn: &mut AsyncPgConnection) -> Result<Vec<PostResponse>, ApiError> {
    Ok(PostRepository::find_all(conn).await?.into_iter().map(PostResponse::from).collect())
}

pub async fn get_by_id(conn: &mut AsyncPgConnection, id: Uuid) -> Result<PostResponse, ApiError> {
    PostRepository::find_by_id(conn, id)
        .await?
        .map(PostResponse::from)
        .ok_or_else(|| ApiError::NotFound(format!("Post {} not found", id)))
}

pub async fn create(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    payload: CreatePostRequest,
) -> Result<PostResponse, ApiError> {
    Ok(PostResponse::from(PostRepository::create(conn, user_id, payload).await?))
}

pub async fn update(
    conn: &mut AsyncPgConnection,
    id: Uuid,
    user_id: Uuid,          // pour vérification ownership
    payload: UpdatePostRequest,
) -> Result<PostResponse, ApiError> {
    // Vérification existence
    let post = PostRepository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Post {} not found", id)))?;

    // Vérification ownership
    if post.user_id != user_id {
        return Err(ApiError::Authorization("You don't own this post".into()));
    }

    Ok(PostResponse::from(PostRepository::update(conn, id, payload).await?))
}

pub async fn delete(
    conn: &mut AsyncPgConnection,
    id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {
    let post = PostRepository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Post {} not found", id)))?;

    if post.user_id != user_id {
        return Err(ApiError::Authorization("You don't own this post".into()));
    }

    PostRepository::delete(conn, id).await?;
    Ok(())
}
```

### handler.rs

```rust
use axum::{Extension, Json, extract::Path, extract::State, http::StatusCode};
use uuid::Uuid;
use utoipa::OpenApi;
use crate::config::state::AppState;
use crate::modules::auth::helpers::Claims;
use crate::core::errors::ApiError;
use crate::core::validator::ValidatedJson;
use super::{dto::*, service};

#[utoipa::path(
    get,
    path = "/api/posts",
    tag = "posts",
    responses(
        (status = 200, description = "List all posts", body = Vec<PostResponse>),
    )
)]
pub async fn get_all(
    State(state): State<AppState>,
) -> Result<Json<Vec<PostResponse>>, ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::get_all(&mut conn).await?))
}

#[utoipa::path(
    post,
    path = "/api/posts",
    tag = "posts",
    security(("bearer_auth" = [])),
    request_body = CreatePostRequest,
    responses(
        (status = 201, description = "Post created", body = PostResponse),
        (status = 422, description = "Validation error"),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,          // injecté par require_auth
    ValidatedJson(payload): ValidatedJson<CreatePostRequest>,  // validation auto
) -> Result<(StatusCode, Json<PostResponse>), ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok((StatusCode::CREATED, Json(service::create(&mut conn, claims.sub, payload).await?)))
}

// get_by_id, update, delete suivent le même pattern...
```

### mod.rs (routes)

```rust
use axum::{Router, routing::{delete, get, post, put}};
use axum::middleware::from_fn_with_state;
use crate::config::state::AppState;
use crate::core::middlewares::{auth::require_auth, rate_limit::rate_limit_by_user};

pub mod dto;
pub mod handler;
pub mod service;

pub fn routes(state: AppState) -> Router {
    Router::new()
        // Routes publiques (optionnel)
        .route("/posts", get(handler::get_all))
        // Routes protégées groupées
        .route("/posts", post(handler::create))
        .route("/posts/{id}", get(handler::get_by_id)
            .put(handler::update)
            .delete(handler::delete))
        .route_layer(from_fn_with_state(state.clone(), rate_limit_by_user))
        .route_layer(from_fn_with_state(state.clone(), require_auth))
        .with_state(state)
}
```

---

## 9. Auth — helpers.rs

```rust
// modules/auth/helpers.rs
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,       // user_id
    pub email: String,
    pub exp: usize,      // expiration timestamp
    pub iat: usize,      // issued at timestamp
}

pub fn create_token(user_id: Uuid, email: &str, secret: &str, expiration: i64) -> Result<String, ApiError> {
    let now = Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        email: email.to_string(),
        exp: (Utc::now() + Duration::seconds(expiration)).timestamp() as usize,
        iat: now,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| ApiError::Internal(e.to_string()))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, ApiError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 0;  // Important pour les tests avec tokens expirés
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
        .map(|data| data.claims)
        .map_err(|e| ApiError::Authentication(e.to_string()))
}

// bcrypt cost adapté à l'environnement
pub fn hash_password(password: &str) -> Result<String, ApiError> {
    let cost = if cfg!(debug_assertions) { 4 } else { 12 };
    bcrypt::hash(password, cost).map_err(|e| ApiError::Internal(e.to_string()))
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    // IMPORTANT : bcrypt est bloquant → spawn_blocking en prod si nécessaire
    bcrypt::verify(password, hash).map_err(|e| ApiError::Internal(e.to_string()))
}
```

---

## 10. Middleware Auth

```rust
// core/middlewares/auth.rs
pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Authentication("Missing token".into()))?;

    let claims = verify_token(token, &state.config.jwt_secret)?;
    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}
```

---

## 11. Rate Limiting

```rust
// core/middlewares/rate_limit.rs

// Par IP — pour les routes publiques (login, register)
pub async fn rate_limit_by_ip(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Lire l'IP depuis les extensions (ConnectInfo) avec fallback
    let ip = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip().to_string())
        .unwrap_or_else(|| "127.0.0.1".to_string());

    check_rate_limit(&state.rate_limit, &ip)?;
    Ok(next.run(request).await)
}

// Par user_id — pour les routes protégées
pub async fn rate_limit_by_user(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    check_rate_limit(&state.rate_limit, &claims.sub.to_string())?;
    Ok(next.run(request).await)
}
```

**IMPORTANT :** le router principal doit utiliser `into_make_service_with_connect_info::<SocketAddr>()` pour que `ConnectInfo` soit disponible :

```rust
// server/server.rs
axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
    .await?;
```

---

## 12. Refresh Token

```rust
// Cookie HttpOnly avec rotation à chaque refresh
let cookie = Cookie::build(("refresh_token", refresh_token))
    .http_only(true)
    .secure(!cfg!(debug_assertions))    // HTTPS en prod, HTTP en dev
    .same_site(SameSite::Strict)
    .path("/api/auth/refresh")
    .max_age(time::Duration::seconds(config.jwt_refresh_expiration))
    .build();
```

---

## 13. ValidatedJson extractor

```rust
// core/validator.rs
pub struct ValidatedJson<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        value.validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        Ok(ValidatedJson(value))
    }
}
```

---

## 14. Migrations SQL

### Migration initiale (générée par Diesel)

```sql
-- 00000000000000_diesel_initial_setup/up.sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE OR REPLACE FUNCTION trigger_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

### Users

```sql
-- create_users/up.sql
CREATE TABLE IF NOT EXISTS users (
    id         UUID      PRIMARY KEY DEFAULT uuid_generate_v4(),
    email      VARCHAR   NOT NULL UNIQUE,
    password   VARCHAR   NOT NULL,
    name       VARCHAR   NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email ON users(email);

CREATE TRIGGER set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
```

### Posts

```sql
-- create_posts/up.sql
CREATE TABLE IF NOT EXISTS posts (
    id         UUID      PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title      VARCHAR   NOT NULL,
    content    TEXT      NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_posts_user_id ON posts(user_id);

CREATE TRIGGER set_updated_at BEFORE UPDATE ON posts
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
```

### Template migration pour nouveaux modules

```sql
CREATE TABLE IF NOT EXISTS <nom_pluriel> (
    id         UUID      PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- colonnes spécifiques ici
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_<nom>_user_id ON <nom_pluriel>(user_id);

CREATE TRIGGER set_updated_at BEFORE UPDATE ON <nom_pluriel>
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
```

---

## 15. Tests e2e (Cucumber)

Les tests e2e sont désormais pilotés par **Cucumber** (features Gherkin + steps Rust), et non plus par des fichiers `e2e_*_tests.rs`.

### Organisation des tests

```tree
tests/
├── cucumber.rs             ← point d'entrée qui lance les features
├── common/
│   └── mod.rs              ← app de test, reset DB, seed helpers
├── features/
│   ├── auth.feature
│   ├── post.feature
│   └── user.feature
└── steps/
    ├── auth_steps.rs       ← World + steps auth
    ├── post_steps.rs       ← World + steps posts
    └── user_steps.rs       ← World + steps users
```

### Orchestrateur Cucumber (tests/cucumber.rs)

```rust
mod steps {
    pub mod auth_steps;
    pub mod post_steps;
    pub mod user_steps;
}
mod common;

use cucumber::World;
use crate::{
    common::reset_db,
    steps::{auth_steps::AuthWorld, post_steps::PostsWorld, user_steps::UsersWorld},
};

#[tokio::main]
async fn main() {
    // AUTH
    reset_db();
    AuthWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run("tests/features/auth.feature")
        .await;

    // POSTS
    reset_db();
    PostsWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run("tests/features/post.feature")
        .await;

    // USERS
    reset_db();
    UsersWorld::cucumber()
        .max_concurrent_scenarios(1)
        .run("tests/features/user.feature")
        .await;
}
```

### configstructure partagée (tests/common/mod.rs)

```rust
use tokio::sync::OnceCell;
use std::sync::Mutex;

static TEST_APP: OnceCell<Router> = OnceCell::const_new();
static DB_LOCK: Mutex<()> = Mutex::new(());

pub async fn get_test_app() -> &'static Router {
    TEST_APP
        .get_or_init(|| async { build_test_app().await })
        .await
}

pub fn reset_db() {
    let _guard = DB_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // revert_all_migrations + run_pending_migrations sur DATABASE_TEST_URL
}

pub async fn build_test_app() -> Router {
    let config = test_config();
    let pool = test_pool().await;

    let state = AppState {
        pool,
        config: Arc::new(config),
        rate_limit: RateLimitStore::new(1000, 1000),
    };

    Router::new().nest("/api", create_router(state))
}

pub async fn seed_user_and_login(app: &Router, email: &str, password: &str) -> String {
    // 1. POST /api/auth/register
    // 2. POST /api/auth/login
    // 3. Retourner access_token
}
```

### Structure d'un step file (exemple)

```rust
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct AuthWorld {
    app: Router,
    status: u16,
    body: serde_json::Value,
}

#[given("la base de données est réinitialisée")]
async fn reset_database(_world: &mut AuthWorld) {
    reset_db();
}

#[when(expr = "je me connecte avec:")]
async fn when_login_with(world: &mut AuthWorld, step: &Step) {
    // construit le payload depuis la table Gherkin
    // envoie la requête HTTP avec app.oneshot(...)
    // stocke status/body dans le world
}

#[then(expr = "le statut de la réponse est {int}")]
async fn then_status_is(world: &mut AuthWorld, expected: u16) {
    assert_eq!(world.status, expected);
}
```

### Commandes tests

```bash
# Exécuter les scénarios e2e Cucumber
cargo test --test cucumber

# Exécuter tous les tests (unit + intégration + cucumber)
cargo test -- --test-threads=1

# Coverage HTML
cargo llvm-cov --all --html --open -- --test-threads=1

# Coverage lcov (CI)
cargo llvm-cov --all -- --test-threads=1
cargo llvm-cov report --lcov --output-path lcov.info
```

### .cargo/config.toml

```toml
[test]
test-threads = 1  # Obligatoire — les migrations partagent la même DB
```

### Problèmes connus et solutions

| Problème | Cause | Solution |
| -------- | ----- | -------- |
| `Cannot start runtime from within runtime` | `OnceLock` sync dans async | Utiliser `tokio::sync::OnceCell` |
| `PoisonError` sur DB_LOCK | Panic dans un test précédent | `unwrap_or_else(\|e\| e.into_inner())` |
| Deadlock migrations | Features/scénarios exécutés en parallèle | `max_concurrent_scenarios(1)` + `DB_LOCK` mutex |
| `ConnectInfo` manquant | Pas de `with_connect_info` sur le router de test | Lire depuis extensions avec `.get()` + fallback `127.0.0.1` |
| Routes 404 | Module non branché dans router.rs | Vérifier `create_router()` + `pub mod` dans `mod.rs` |

---

## 16. Générateur de modules (scaffold)

```bash
# Générer un nouveau module
make module-gen invoice

# Supprimer un module
make module-del invoice
```

**Ce que le générateur crée automatiquement :**

- `migrations/TIMESTAMP_create_invoices/up.sql` + `down.sql`
- `src/db/invoice/model.rs` — structs Diesel
- `src/db/invoice/repository.rs` — avec macros
- `src/db/invoice/mod.rs`
- `src/modules/invoice/dto.rs` — Response + Create + Update
- `src/modules/invoice/service.rs` — fonctions libres
- `src/modules/invoice/handler.rs` — handlers utoipa
- `src/modules/invoice/mod.rs` — fn routes()

**Ce qu'il faut faire ensuite manuellement :**

1. Compléter `up.sql` avec les colonnes métier
2. `diesel migration run`
3. Ajouter `pub mod invoice;` dans `src/db/mod.rs`
4. Ajouter `pub mod invoice;` dans `src/modules/mod.rs`
5. Brancher dans `src/server/router.rs` : `.merge(invoice::routes(state.clone()))`
6. Ajouter les paths utoipa dans `src/server/swagger.rs`

**Nommage automatique :**

- CLI input : `invoice` (snake_case)
- Struct Rust : `Invoice`, `NewInvoice`, `InvoiceRepository`, `InvoiceResponse`...
- Table SQL : `invoices` (pluriel automatique)
- Route : `/api/invoices`

---

## 17. CI/CD

### build.yml — pipeline CI

```yaml
# Triggers : push/PR sur main et develop
# Jobs : Build & Analyze (lint + tests + coverage + SonarCloud + cargo audit)

steps:
  - uses: actions/checkout@v4
    with:
      fetch-depth: 0        # Obligatoire SonarCloud

  - name: Install Rust stable
    uses: dtolnay/rust-toolchain@stable
    with:
      components: rustfmt, clippy

  - name: Cache Cargo
    uses: Swatinem/rust-cache@v2

  - name: Install cargo-llvm-cov
    uses: taiki-e/install-action@cargo-llvm-cov

  - name: Install diesel CLI
    run: cargo install diesel_cli --no-default-features --features postgres

  - name: Run migrations
    run: diesel migration run

  - name: Run tests with coverage
    run: cargo llvm-cov --all -- --test-threads=1

  - name: Generate lcov report
    run: cargo llvm-cov report --lcov --output-path lcov.info

  - name: SonarCloud Scan
    uses: SonarSource/sonarqube-scan-action@v6
    with:
      args: >
        -Dsonar.rust.lcov.reportPaths=/home/runner/work/rust-api-template/rust-api-template/lcov.info
    env:
      GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}
      SONAR_HOST_URL: https://sonarcloud.io

  - name: Security audit
    run: |
      cargo install cargo-audit
      cargo audit
```

### cd.yml — pipeline CD

```yaml
# Trigger : push sur main uniquement
# Jobs :
#   1. build — Docker multi-stage → Docker Hub (latest + sha-)
#   2. deploy — curl sur RENDER_DEPLOY_HOOK_URL
```

### Dockerfile — multi-stage avec cargo-chef

```dockerfile
FROM rust:1.85-slim-bookworm AS chef
RUN apt-get update && apt-get install -y libpq-dev pkg-config curl && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json    # cache des dépendances
COPY . .
RUN cargo build --release --bin server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y libpq5 ca-certificates curl && rm -rf /var/lib/apt/lists/*
RUN useradd --uid 1001 --no-create-home --shell /bin/false appuser
WORKDIR /app
COPY --from=builder /app/target/release/server /app/server
COPY --from=builder /app/migrations /app/migrations
RUN chown -R appuser:appuser /app
USER appuser
ENV SERVER_HOST=0.0.0.0 SERVER_PORT=8080 RUST_LOG=info
EXPOSE 8080
CMD ["/app/server"]
```

**IMPORTANT :** `curl` doit être dans le stage `chef` (hérité par `builder`) car `utoipa-swagger-ui` le télécharge pendant la compilation.

### sonar-project.properties

```properties
sonar.projectKey=<org>_rust-api-template
sonar.organization=<org>
sonar.projectName=rust-api-template
sonar.sources=src
sonar.exclusions=src/bin/**,**/schema.rs
sonar.host.url=https://sonarcloud.io
sonar.rust.lcov.reportPaths=lcov.info    # NE PAS utiliser sonar.lcov.reportPaths ni sonar.coverageReportPaths
```

### audit.toml

```toml
[advisories]
ignore = ["RUSTSEC-2023-0071"]
# Marvin Attack sur rsa 0.9.x via jsonwebtoken 10.x
# Non affecté : l'application utilise uniquement HS256 (HMAC), pas RSA
```

### Secrets GitHub requis

| Secret | Description |
| ------ | ----------- |
| `DOCKERHUB_USERNAME` | Username Docker Hub |
| `DOCKERHUB_TOKEN` | Token Read & Write (pas Read only) |
| `RENDER_DEPLOY_HOOK_URL` | Render → Settings → Deploy Hook (URL complète avec https://) |
| `SONAR_TOKEN` | SonarCloud → My Account → Security |

### render.yaml (Blueprint)

```yaml
services:
  - type: web
    name: rust-api-template
    runtime: docker
    dockerfilePath: ./Dockerfile
    region: frankfurt
    plan: free
    healthCheckPath: /api/health
    envVars:
      - key: SERVER_HOST
        value: 0.0.0.0
      - key: SERVER_PORT
        value: 8080
      - key: DATABASE_URL
        fromDatabase:
          name: rust-api-db
          property: connectionString
      - key: JWT_SECRET
        sync: false
      # JWT_EXPIRATION, JWT_REFRESH_SECRET, JWT_REFRESH_EXPIRATION, FRONTEND_URL, ENVIRONMENT, RUST_LOG
      # → à configurer manuellement dans le dashboard Render

databases:
  - name: rust-api-db
    plan: free
    region: frankfurt
    databaseName: app_db
    # NE PAS mettre le champ "user" — non supporté par Blueprint Render
```

---

## 18. Variables d'environnement

```env
# Serveur
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
ENVIRONMENT=development          # development | production

# Base de données
DATABASE_URL=postgres://postgres:password@localhost:5432/app_db
DATABASE_TEST_URL=postgres://postgres:password@localhost:5433/app_test_db

# JWT
JWT_SECRET=<base64 64 bytes>
JWT_EXPIRATION=86400             # 24h en secondes
JWT_REFRESH_SECRET=<base64 64 bytes>
JWT_REFRESH_EXPIRATION=604800    # 7j en secondes

# App
FRONTEND_URL=http://localhost:5173
RUST_LOG=info
```

**Générer un secret JWT robuste (PowerShell) :**

```powershell
[System.Convert]::ToBase64String([System.Security.Cryptography.RandomNumberGenerator]::GetBytes(64))
```

---

## 19. docker-compose.yml (développement local)

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: app_db
    ports:
      - "5432:5432"

  postgres_test:
    image: postgres:16
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: password
      POSTGRES_DB: app_test_db
    ports:
      - "5433:5432"
```

---

## 20. Pièges connus et solutions

| Piège | Mauvais | Correct |
| ----- | ------- | ------- |
| Paramètres de route Axum 0.8 | `.route("/posts/:id")` | `.route("/posts/{id}")` |
| Feature utoipa | `axum_extras = false`, `actix_extras = true` | `axum_extras` uniquement |
| Macro Diesel dans macro_rules | `$table::table` | `use $table as _table; _table::table` |
| ConnectInfo en tests | Extractor dans signature | `.extensions().get::<ConnectInfo<...>>()` + fallback |
| OnceCell pour test app | `std::sync::OnceLock` | `tokio::sync::OnceCell` |
| PoisonError mutex | `.unwrap()` | `.unwrap_or_else(\|e\| e.into_inner())` |
| SonarCloud coverage Rust | `sonar.coverageReportPaths` | `sonar.rust.lcov.reportPaths` |
| Docker Hub 401 | Token Read only | Token Read & Write |
| Render Blueprint user DB | `user: postgres` | Supprimer le champ `user` |
| cargo-chef + utoipa-swagger-ui | `curl` absent | Ajouter `curl` dans le stage `chef` |
| edition 2024 en Docker | `FROM rust:1.76` | `FROM rust:1.85` minimum |
