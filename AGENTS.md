# AGENTS.md

Instructions pour les agents IA qui travaillent sur ce dépôt. Le `README.md` s'adresse aux humains ;
ce fichier contient ce qu'il faut savoir pour modifier le code correctement du premier coup.

API REST Rust : **Axum 0.8 · Diesel 2.4 + diesel-async (PostgreSQL 18) · utoipa (OpenAPI) · JWT**.
Edition 2024, MSRV `rust-version = "1.89"` (`Cargo.toml`).

## Commandes

```bash
docker compose up -d                       # PostgreSQL dev (5432) + test (5433)
cp .env.example .env                       # une seule fois
cargo run                                  # API sur :8080, Swagger sur /swagger-ui, migrations appliquées au démarrage

cargo fmt --check                          # ┐
cargo clippy --all-targets -- -D warnings  # │ ce que vérifie la CI (bloquant)
cargo test                                 # ┘ unitaires + tests/*.rs + Cucumber (base de test requise)
cargo audit                                # vulnérabilités (non bloquant en CI)

diesel migration generate create_<table>s  # nouvelle migration (diesel_cli --features postgres-bundled)
diesel migration run                       # applique et régénère src/db/schema.rs
```

**Une tâche est terminée quand** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` et
`cargo test` passent, et que le comportement ajouté est couvert par un test.

## Architecture

```
src/
├── main.rs  lib.rs  config.rs   config.rs : toute la configuration (variables d'env)
├── app/      assemblage et démarrage : router (mod.rs), server.rs, openapi.rs, telemetry.rs
├── common/   partagé : error.rs, extract.rs, jwt.rs, pagination.rs, state.rs, middlewares/
├── db/       pool + migrations (mod.rs), repository.rs (générique), schema.rs, <table>/{model,repository}.rs
└── modules/  fonctionnalités HTTP : <module>/{mod.rs, handler.rs, service.rs, dto.rs}
```

### Règles de dépendance (à respecter strictement)

- `app → modules → (common, db)` · `db → common` · `common → config`.
- **Rien n'importe `app`** (sauf `main.rs` et les tests).
- **`common` n'importe jamais un module.** Si du code de module devient partagé, il descend dans `common`.
- **`db` ignore le HTTP** (pas d'axum, pas de DTO). Une table peut exister sans module, et inversement.
- Un module n'importe pas un autre module. Exception existante : `user` utilise `auth::helpers` (bcrypt).

### Rôle de chaque couche dans un module

| Fichier | Contient | Ne contient pas |
|---------|----------|-----------------|
| `mod.rs` | `pub fn routes() -> OpenApiRouter<AppState>` | logique |
| `handler.rs` | extraction (State, AuthUser, Validated*), appel du service, code HTTP | SQL, règles métier |
| `service.rs` | règles métier, vérification de propriété, conversion modèle → DTO | types axum |
| `dto.rs` | structs requête (`Deserialize + Validate + ToSchema`) et réponse (`Serialize + ToSchema`) | modèles Diesel |

Module de référence à copier : **`modules/post`** (+ `db/post`). Il couvre CRUD, pagination,
authentification et vérification du propriétaire.

## Recettes

### Ajouter un endpoint ou un module

1. `src/modules/<module>/` avec `mod.rs`, `handler.rs`, `service.rs`, `dto.rs` (même format que `post`).
2. `pub mod <module>;` dans `src/modules/mod.rs`.
3. `.merge(<module>::routes())` dans `api_router()` (`src/app/mod.rs`).
4. Si le module introduit un nouveau `tag` OpenAPI, ajouter sa description dans `tags(...)` (`src/app/openapi.rs`).

Le chemin, la méthode **et** la doc OpenAPI viennent de `#[utoipa::path]` ; il n'y a rien à déclarer ailleurs.

```rust
// handler.rs — chemin SANS le préfixe /api (ajouté par app/mod.rs)
#[utoipa::path(
    post, path = "/posts", tag = "posts",
    security(("bearer_auth" = [])),
    request_body = CreatePostRequest,
    responses(
        (status = 201, body = PostResponse),
        (status = 401, body = ErrorResponse),
        (status = 422, description = "Données invalides (VALIDATION_FAILED)", body = ErrorResponse),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,                                   // route protégée
    ValidatedJson(payload): ValidatedJson<CreatePostRequest>, // corps : toujours en dernier
) -> Result<(StatusCode, Json<PostResponse>), ApiError> {
    let mut conn = state.pool.get().await?;
    let post = service::create(&mut conn, user.id, payload).await?;
    Ok((StatusCode::CREATED, Json(post)))
}

// mod.rs — un routes!(...) par chemin (les handlers d'un même chemin sont groupés)
pub fn routes() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handler::get_all, handler::create))                    // /posts
        .routes(routes!(handler::get_by_id, handler::update, handler::delete)) // /posts/{id}
}
```

Ordre des paramètres d'un handler : `State`, `AuthUser`, `ValidatedPath`, `ValidatedQuery`, puis le corps
(`ValidatedJson`) **en dernier** (contrainte d'axum).

### Ajouter une table

1. `diesel migration generate create_<table>s`, puis écrire `up.sql` / `down.sql` :

   ```sql
   CREATE TABLE comments (
       id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
       post_id    UUID        NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
       content    TEXT        NOT NULL,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
   );
   CREATE INDEX idx_comments_post_id_created_at ON comments (post_id, created_at DESC);
   SELECT diesel_manage_updated_at('comments'); -- met à jour updated_at à chaque UPDATE
   ```
   ```sql
   DROP TABLE IF EXISTS comments;
   ```
   Conventions : UUID, `TIMESTAMPTZ` (jamais `TIMESTAMP`), `diesel_manage_updated_at` sur chaque table
   ayant `updated_at`, index sur les clés étrangères.
2. `diesel migration run` → `src/db/schema.rs` est régénéré (ne pas l'éditer à la main).
3. `src/db/<table>/` avec `mod.rs`, `model.rs`, `repository.rs` (copier `db/post`), puis `pub mod <table>;`
   dans `src/db/mod.rs`. Dates : `chrono::DateTime<Utc>`.

**Ne jamais modifier une migration déjà appliquée** sur un environnement partagé : créer une nouvelle
migration. Une migration modifiée localement impose de vider la base locale.

### Repository : méthodes génériques

Chaque capacité est un trait implémenté par une macro. N'implémenter que ce qui est utilisé.

```rust
pub struct CommentRepository;
crate::impl_find_by_id!(CommentRepository, Comment, crate::db::schema::comments, Uuid);
crate::impl_delete!(CommentRepository, crate::db::schema::comments, Uuid);
crate::impl_paginate!(CommentRepository, Comment, crate::db::schema::comments, created_at);
// impl_base_repository! = find_all + find_by_id + delete ; aussi impl_exists!, impl_count!, impl_soft_delete!
```

- Pour **appeler** ces méthodes : `use crate::db::repository::prelude::*;` (sinon erreur « method not found »).
- Méthodes spécifiques (insert, update, recherche par email, jointures) : bloc `impl` classique dans le même fichier.
- Pagination filtrée : `Repo::find_paginated_by(conn, params, || Box::new(comments::post_id.eq(id)))`.

### Erreurs

Toutes les erreurs passent par `ApiError` (`src/common/error.rs`) et sont renvoyées au format
RFC 9457 avec un `code` stable. **Ajouter un code = une ligne** dans `error_codes!` :

```rust
CommentNotFound => (NOT_FOUND, "Comment not found"),
```

```rust
let post = Repo::find_by_id(conn, id).await?.ok_or(ErrorCode::PostNotFound)?;              // code seul
return Err(ApiError::new(ErrorCode::Forbidden).with_detail("You can only edit your own posts"));
let hash = bcrypt::hash(pwd, cost).map_err(ApiError::internal)?; // cause loggée, jamais envoyée au client
```

- Les erreurs Diesel, du pool et de `validator` se convertissent avec `?`.
- Ne jamais mettre d'information interne (SQL, chemin, message d'une lib) dans `with_detail`.
- Pas de `unwrap()` / `expect()` hors tests et démarrage.
- Codes HTTP : validation → 422 `VALIDATION_FAILED`, JSON mal formé → 400, ressource absente → 404 `<X>_NOT_FOUND`.

### Authentification, propriété, validation, pagination

- **Route protégée** : ajouter `user: AuthUser` au handler (`user.id` = UUID de l'utilisateur).
- **Propriété** : vérifier dans le service et renvoyer `ErrorCode::Forbidden` (voir `ensure_owner` dans `post/service.rs`).
- **Validation** : DTO avec `#[derive(Validate)]` + `#[serde(deny_unknown_fields)]`, extrait via
  `ValidatedJson` / `ValidatedQuery` / `ValidatedPath` (`UuidParam` pour `{id}`). Messages de validation en anglais.
- **Pagination** : `ValidatedQuery<PaginationQuery>` → `PaginationParams::from(...)` → repository
  `find_paginated_by` → `PaginatedResponse<T>` (`.map(Dto::from)` pour convertir).
- **Ne jamais exposer** un modèle Diesel directement : passer par un DTO de réponse (`impl From<Model> for Dto`).

### Configuration

Ajouter une variable : champ dans la struct concernée de `src/config.rs`, lecture via `env.or("NOM", défaut)`
(ou `env.required("NOM")`), documentation dans `.env.example`, test dans `config.rs`.
Ne jamais lire `std::env::var` ailleurs que dans `config.rs`.

## Tests

| Type | Où | Base de données |
|------|----|-----------------|
| Unitaires | `#[cfg(test)] mod tests` dans le fichier | non |
| Repository | `tests/repository.rs` | transaction annulée (`test_conn()`), parallélisable |
| Santé | `tests/health.rs` | router complet via `common::test_state()` |
| E2E | `tests/features/*.feature` + `tests/steps/*_steps.rs` (Cucumber) | réinitialisée à chaque scénario |

Ajouter des scénarios E2E pour un module :

1. `tests/features/<module>.feature` (étapes en français, `Background: Given la base de données est réinitialisée`).
2. `tests/steps/<module>_steps.rs`, déclaré dans le bloc `mod steps` de `tests/cucumber.rs`.
3. Utiliser `ApiWorld` (`tests/steps/world.rs`) : `send`, `send_as_current`, `login_as`, `user_id_of`, `last_id`,
   `table_json(step)`.
4. Steps communs déjà disponibles (`common_steps.rs`) : connexion (`je suis connecté en tant que …`),
   `le statut de la réponse est {int}`, `la réponse contient:` (table champ/valeur, ex. `| code | POST_NOT_FOUND |`),
   `la réponse contient un champ …`, `le champ … de la réponse vaut …`, `la réponse signale une erreur sur le champ …`.

Un step sans implémentation fait échouer le run (`fail_on_skipped`).

## Pièges connus

- **Chemins utoipa** : sans `/api` (`path = "/posts"`), le préfixe est ajouté par `nest("/api", …)`.
- **Crates 0.x** (diesel-async, utoipa-axum…) : une montée « mineure » (0.10 → 0.11) peut être cassante.
- **MSRV 1.89** : pas d'API de la std plus récente (clippy `incompatible_msrv` le signale).
- **`schema.rs`** est généré : toute modification passe par une migration.
- **libpq n'est pas nécessaire** pour l'API ni les tests (diesel-async) ; seul `diesel_cli` l'embarque (`postgres-bundled`).
- **Logs** : `tracing` uniquement (`tracing::info!`…), jamais `println!`. Les 5xx sont déjà loggées par `ApiError`.

## Conventions

- Code et identifiants en anglais ; commentaires et doc (`///`, `//!`) en français ; messages d'erreur et de
  validation de l'API en anglais.
- Commits : anglais, minuscules, impératif, un sujet par commit (ex. `add comment module`).
- Pas de nouvelle dépendance sans nécessité ; vérifier sa compatibilité MSRV.
