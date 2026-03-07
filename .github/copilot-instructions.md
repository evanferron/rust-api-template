# Copilot Instructions — rust-api-template

## Stack technique

- **Framework** : Axum 0.8 (`{id}` et non `:id` pour les paramètres de route)
- **ORM** : Diesel 2 + diesel-async 0.7 + bb8 (pool async PostgreSQL)
- **Auth** : JWT (jsonwebtoken 10) — algorithme HS256 uniquement, refresh token en cookie HttpOnly
- **Validation** : validator 0.19 via l'extractor custom `ValidatedJson<T>`
- **Logs** : tracing + tracing-subscriber (pretty en dev, JSON en prod)
- **Rate limiting** : governor + dashmap (en mémoire, par IP et par user)
- **Doc API** : utoipa 5 + utoipa-swagger-ui 9 (feature `axum_extras`, pas `actix_extras`)
- **Edition Rust** : 2024

## Architecture

```
src/
├── core/          ← erreurs, logger, middlewares, repository générique, validator
├── db/            ← schema.rs (généré Diesel), modèles et repositories par entité
├── infra/         ← Config::from_env(), AppState (pool + config + rate_limit)
├── launch/        ← router.rs (assemblage des routes), swagger.rs (OpenAPI)
├── modules/       ← auth, health, user, post — chacun a dto/handler/service/mod
├── bin/
│   └── generate.rs ← CLI scaffold pour générer de nouveaux modules
├── lib.rs
└── main.rs
```

## AppState

```rust
pub struct AppState {
    pub pool: Pool<AsyncPgConnection>,  // bb8
    pub config: Config,
    pub rate_limit: RateLimitStore,
}
```

Pas de services ni repositories dans l'AppState — fonctions libres uniquement.

## Pattern repository

Chaque repository est une struct vide avec des méthodes statiques async.
Les macros génèrent les méthodes courantes :

```rust
pub struct PostRepository;

// Génère : find_all, find_by_id, delete + trait BaseRepository
crate::impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);

// Optionnel à la carte
crate::impl_exists!(PostRepository, crate::db::schema::posts, Uuid);
crate::impl_count!(PostRepository, crate::db::schema::posts);
crate::impl_find_paginated!(PostRepository, Post, crate::db::schema::posts, created_at);
crate::impl_soft_delete!(PostRepository, Post, crate::db::schema::posts, Uuid);

impl PostRepository {
    // Méthodes spécifiques uniquement
    pub async fn find_by_user_id(...) { ... }
    pub async fn create(...) { ... }
    pub async fn update(...) { ... }
}
```

## Pattern handler

```rust
pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,      // injecté par require_auth
    ValidatedJson(payload): ValidatedJson<CreatePostRequest>, // validation auto
) -> Result<(StatusCode, Json<PostResponse>), ApiError> {
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let item = service::create(&mut conn, claims.sub, payload).await?;
    Ok((StatusCode::CREATED, Json(item)))
}
```

## Pattern service

Fonctions libres (pas de struct Service) :

```rust
pub async fn create(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    payload: CreatePostRequest,
) -> Result<PostResponse, ApiError> {
    let item = PostRepository::create(conn, user_id, payload).await?;
    Ok(PostResponse::from(item))
}
```

## Gestion des erreurs

`ApiError` est l'erreur centrale — elle implémente `IntoResponse` :

```rust
ApiError::NotFound(String)
ApiError::Conflict(String)
ApiError::Authentication(String)
ApiError::Authorization(String)
ApiError::Validation(String)
ApiError::Internal(String)
ApiError::RateLimitExceeded { ... }
```

## DTO pattern

- `XxxResponse` implémente `From<Model>`
- `CreateXxxRequest` et `UpdateXxxRequest` dérivent `Validate`
- `UpdateXxxRequest` utilise `Option<T>` pour les champs optionnels
- `#[serde(deny_unknown_fields)]` sur tous les requests

## Middlewares

- `require_auth` — extrait le Bearer token, injecte `Claims` dans les extensions
- `rate_limit_by_ip` — routes publiques (auth)
- `rate_limit_by_user` — routes protégées

## Ajout d'un nouveau module

```bash
cargo run --bin generate -- generate <nom>
# ex: cargo run --bin generate -- generate invoice
```

Génère automatiquement : migration, model, repository, dto, service, handler, mod, routes.

Ensuite :

1. Compléter `up.sql` avec les colonnes
2. `diesel migration run`
3. Ajouter `pub mod <nom>;` dans `src/db/mod.rs` et `src/modules/mod.rs`
4. Brancher les routes dans `src/launch/router.rs`

## Relations Diesel (One-to-Many)

```rust
// model.rs
#[derive(Associations)]
#[diesel(belongs_to(User))]
pub struct Post { ... }

// schema.rs
diesel::joinable!(posts -> users (user_id));
diesel::allow_tables_to_appear_in_same_query!(users, posts);
```

## Tests e2e

```rust
// Utilise get_test_app() — instance partagée via OnceCell
let app = get_test_app().await.clone();
// reset_db() au début de chaque test
reset_db();
// Lance avec --test-threads=1
```

## CI/CD

- **CI** : `.github/workflows/build.yml` — lint + tests + SonarCloud + cargo audit
- **CD** : `.github/workflows/cd.yml` — Docker Hub + Render deploy hook
- **Coverage** : cargo-llvm-cov → lcov.info → `sonar.rust.lcov.reportPaths`

## Conventions importantes

- Routes Axum 0.8 : `{id}` pas `:id`
- Macros `#[macro_export]` → accessibles via `crate::impl_xxx!`
- `ConnectInfo<SocketAddr>` requis pour rate_limit_by_ip → `into_make_service_with_connect_info`
- Cookie refresh token : `SameSite::Strict`, `.secure(!cfg!(debug_assertions))`
- bcrypt cost : `cfg!(debug_assertions)` → 4 en dev, 12 en prod
- Pagination : `PaginationParams::new(page, per_page)` avec clamp 1-100
