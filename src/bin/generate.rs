use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.as_slice() {
        [_, cmd, name] if cmd == "generate" || cmd == "g" => {
            generate(name);
        }
        [_, cmd, name] if cmd == "delete" || cmd == "d" => {
            delete(name);
        }
        _ => {
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!(
        r#"
scaffold — Générateur de modules pour rust-api-template

USAGE:
    cargo run --bin scaffold -- <COMMANDE> <NOM>

COMMANDES:
    generate, g    Génère un nouveau module
    delete,   d    Supprime un module existant

EXEMPLES:
    cargo run --bin scaffold -- generate invoice
    cargo run --bin scaffold -- generate blog_post
    cargo run --bin scaffold -- delete invoice

Le nom est automatiquement converti :
    snake_case  → pour les fichiers et modules Rust  (ex: blog_post)
    PascalCase  → pour les structs                   (ex: BlogPost)
    kebab-case  → pour les migrations Diesel         (ex: blog-post)
"#
    );
}

// ---------------------------------------------------------------------------
// Génération
// ---------------------------------------------------------------------------

fn generate(name: &str) {
    let module = ModuleNames::from(name);

    println!("\n🚀 Génération du module \"{}\"...\n", module.snake);

    // Vérifie que les dossiers parent existent
    check_project_structure();

    let files = vec![
        // DB layer
        (
            format!("src/db/{}/model.rs", module.snake),
            template_model(&module),
        ),
        (
            format!("src/db/{}/repository.rs", module.snake),
            template_repository(&module),
        ),
        (
            format!("src/db/{}/mod.rs", module.snake),
            template_db_mod(&module),
        ),
        // Module layer
        (
            format!("src/modules/{}/dto.rs", module.snake),
            template_dto(&module),
        ),
        (
            format!("src/modules/{}/service.rs", module.snake),
            template_service(&module),
        ),
        (
            format!("src/modules/{}/handler.rs", module.snake),
            template_handler(&module),
        ),
        (
            format!("src/modules/{}/mod.rs", module.snake),
            template_module_mod(&module),
        ),
        // Migration
        (
            format!(
                "migrations/{}_create_{}/up.sql",
                chrono_prefix(),
                module.snake
            ),
            template_migration_up(&module),
        ),
        (
            format!(
                "migrations/{}_create_{}/down.sql",
                chrono_prefix(),
                module.snake
            ),
            template_migration_down(&module),
        ),
    ];

    let mut created = 0;
    let mut skipped = 0;

    for (path, content) in &files {
        let path = Path::new(path);

        if path.exists() {
            println!("  ⚠️  Existe déjà   {}", path.display());
            skipped += 1;
            continue;
        }

        // Crée les dossiers parents si nécessaire
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("Impossible de créer {}: {}", parent.display(), e));
        }

        fs::write(path, content)
            .unwrap_or_else(|e| panic!("Impossible d'écrire {}: {}", path.display(), e));

        println!("  ✅ Créé           {}", path.display());
        created += 1;
    }

    println!(
        "\n📦 {} fichier(s) créé(s), {} ignoré(s).\n",
        created, skipped
    );
    print_next_steps(&module);
}

// ---------------------------------------------------------------------------
// Suppression
// ---------------------------------------------------------------------------

fn delete(name: &str) {
    let module = ModuleNames::from(name);

    println!("\n🗑️  Suppression du module \"{}\"...\n", module.snake);

    let dirs = vec![
        format!("src/db/{}", module.snake),
        format!("src/modules/{}", module.snake),
    ];

    for dir in &dirs {
        let path = Path::new(dir);
        if path.exists() {
            fs::remove_dir_all(path)
                .unwrap_or_else(|e| panic!("Impossible de supprimer {}: {}", path.display(), e));
            println!("  🗑️  Supprimé       {}", path.display());
        } else {
            println!("  ⚠️  Introuvable    {}", path.display());
        }
    }

    println!("\n⚠️  Les migrations ne sont pas supprimées automatiquement.");
    println!("   Lance : diesel migration revert\n");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn check_project_structure() {
    for dir in &["src/db", "src/modules", "migrations"] {
        if !Path::new(dir).exists() {
            eprintln!(
                "❌ Dossier \"{}\" introuvable. Lance ce script depuis la racine du projet.",
                dir
            );
            std::process::exit(1);
        }
    }
}

fn chrono_prefix() -> String {
    // Format : YYYY-MM-DD-HHmmss
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Conversion manuelle (pas de dépendance chrono dans le bin)
    let (y, mo, d, h, mi, s) = unix_to_datetime(secs);
    format!("{:04}-{:02}-{:02}-{:02}{:02}{:02}", y, mo, d, h, mi, s)
}

fn unix_to_datetime(mut secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    secs /= 60;
    let mi = secs % 60;
    secs /= 60;
    let h = secs % 24;
    secs /= 24;
    let days = secs + 719468;
    let era = days / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d, h, mi, s)
}

fn print_next_steps(module: &ModuleNames) {
    println!("📋 Étapes suivantes :\n");
    println!(
        "  1. Complète la migration  migrations/*_create_{}/up.sql",
        module.snake
    );
    println!("     Lance : diesel migration run\n");
    println!("  2. Mets à jour src/db/schema.rs (auto après migration)\n");
    println!(
        "  3. Ajoute le module DB dans src/db/mod.rs :\n     pub mod {};\n",
        module.snake
    );
    println!(
        "  4. Ajoute le module dans src/modules/mod.rs :\n     pub mod {};\n",
        module.snake
    );
    println!(
        "  5. Branche les routes dans src/launch/router.rs :\n     .merge({}::routes(state.clone()))\n",
        module.snake
    );
}

// ---------------------------------------------------------------------------
// Nommage
// ---------------------------------------------------------------------------

struct ModuleNames {
    /// snake_case  — utilisé pour les modules, fichiers, colonnes DB
    snake: String,
    /// PascalCase  — utilisé pour les structs Rust
    pascal: String,
    /// kebab-case  — utilisé pour les noms de migration Diesel
    kebab: String,
    /// SCREAMING_SNAKE — utilisé pour les constantes
    #[allow(dead_code)]
    upper: String,
}

impl ModuleNames {
    fn from(input: &str) -> Self {
        let snake = to_snake(input);
        let pascal = to_pascal(&snake);
        let kebab = snake.replace('_', "-");
        let upper = snake.to_uppercase();
        Self {
            snake,
            pascal,
            kebab,
            upper,
        }
    }
}

fn to_snake(input: &str) -> String {
    // Accepte snake_case, PascalCase, kebab-case en entrée
    let s = input.replace('-', "_");
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

fn to_pascal(snake: &str) -> String {
    snake
        .split('_')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

fn template_model(m: &ModuleNames) -> String {
    format!(
        r#"use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{{Deserialize, Serialize}};
use uuid::Uuid;

/// Modèle Diesel représentant une ligne de la table `{snake}s`.
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable, Associations)]
#[diesel(table_name = crate::db::schema::{snake}s)]
#[diesel(belongs_to(crate::db::user::model::User))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct {pascal} {{
    pub id: Uuid,
    pub user_id: Uuid,
    // TODO: ajoute tes champs ici
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}}

/// Modèle pour les insertions (INSERT INTO).
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::db::schema::{snake}s)]
pub struct New{pascal} {{
    pub id: Uuid,
    pub user_id: Uuid,
    // TODO: ajoute tes champs ici
}}
"#,
        snake = m.snake,
        pascal = m.pascal,
    )
}

fn template_repository(m: &ModuleNames) -> String {
    format!(
        r#"use diesel::{{ExpressionMethods, QueryDsl, SelectableHelper}};
use diesel_async::{{AsyncPgConnection, RunQueryDsl}};
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::{snake}::model::{{New{pascal}, {pascal}}};
use crate::db::schema::{snake}s::dsl;
use crate::modules::{snake}::dto::{{Create{pascal}Request, Update{pascal}Request}};

// ---------------------------------------------------------------------------
// Macros génériques
// ---------------------------------------------------------------------------

crate::impl_base_repository!({pascal}Repository, {pascal}, crate::db::schema::{snake}s, Uuid);
crate::impl_exists!({pascal}Repository, crate::db::schema::{snake}s, Uuid);
crate::impl_count!({pascal}Repository, crate::db::schema::{snake}s);
crate::impl_find_paginated!({pascal}Repository, {pascal}, crate::db::schema::{snake}s, created_at);

pub struct {pascal}Repository;

// ---------------------------------------------------------------------------
// Méthodes spécifiques à {pascal}
// ---------------------------------------------------------------------------

impl {pascal}Repository {{
    pub async fn find_by_user_id(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
    ) -> Result<Vec<{pascal}>, ApiError> {{
        dsl::{snake}s
            .filter(dsl::user_id.eq(user_id))
            .order(dsl::created_at.desc())
            .load::<{pascal}>(conn)
            .await
            .map_err(ApiError::from)
    }}

    pub async fn create(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
        payload: Create{pascal}Request,
    ) -> Result<{pascal}, ApiError> {{
        let new = New{pascal} {{
            id: Uuid::new_v4(),
            user_id,
            // TODO: mappe les champs du payload
        }};

        diesel::insert_into(dsl::{snake}s)
            .values(&new)
            .returning({pascal}::as_returning())
            .get_result::<{pascal}>(conn)
            .await
            .map_err(ApiError::from)
    }}

    pub async fn update(
        conn: &mut AsyncPgConnection,
        id: Uuid,
        payload: Update{pascal}Request,
    ) -> Result<{pascal}, ApiError> {{
        diesel::update(dsl::{snake}s.find(id))
            .set(&{pascal}Changeset {{
                // TODO: mappe les champs du payload
            }})
            .returning({pascal}::as_returning())
            .get_result::<{pascal}>(conn)
            .await
            .map_err(ApiError::from)
    }}
}}

// ---------------------------------------------------------------------------
// Changeset interne
// ---------------------------------------------------------------------------

#[derive(diesel::AsChangeset)]
#[diesel(table_name = crate::db::schema::{snake}s)]
struct {pascal}Changeset {{
    // TODO: ajoute les champs modifiables
}}
"#,
        snake = m.snake,
        pascal = m.pascal,
    )
}

fn template_db_mod(_m: &ModuleNames) -> String {
    format!("pub mod model;\npub mod repository;\n",)
}

fn template_dto(m: &ModuleNames) -> String {
    format!(
        r#"use chrono::NaiveDateTime;
use serde::{{Deserialize, Serialize}};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::db::{snake}::model::{pascal};

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
pub struct {pascal}Response {{
    pub id: Uuid,
    pub user_id: Uuid,
    // TODO: ajoute tes champs ici
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}}

impl From<{pascal}> for {pascal}Response {{
    fn from(item: {pascal}) -> Self {{
        Self {{
            id: item.id,
            user_id: item.user_id,
            // TODO: mappe les champs ici
            created_at: item.created_at,
            updated_at: item.updated_at,
        }}
    }}
}}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct Create{pascal}Request {{
    // TODO: ajoute les champs de création avec leurs validations
    // Exemple :
    // #[validate(length(min = 1, max = 255))]
    // pub title: String,
}}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, ToSchema, Validate)]
#[serde(deny_unknown_fields)]
pub struct Update{pascal}Request {{
    // TODO: ajoute les champs modifiables (tous Option<T>)
    // Exemple :
    // #[validate(length(min = 1, max = 255))]
    // pub title: Option<String>,
}}
"#,
        snake = m.snake,
        pascal = m.pascal,
    )
}

fn template_service(m: &ModuleNames) -> String {
    format!(
        r#"use diesel_async::AsyncPgConnection;
use uuid::Uuid;

use crate::core::errors::ApiError;
use crate::db::{snake}::repository::{pascal}Repository;
use crate::modules::{snake}::dto::{{Create{pascal}Request, {pascal}Response, Update{pascal}Request}};

pub async fn get_all_by_user(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
) -> Result<Vec<{pascal}Response>, ApiError> {{
    let items = {pascal}Repository::find_by_user_id(conn, user_id).await?;
    Ok(items.into_iter().map({pascal}Response::from).collect())
}}

pub async fn get_by_id(
    conn: &mut AsyncPgConnection,
    id: Uuid,
) -> Result<{pascal}Response, ApiError> {{
    {pascal}Repository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{pascal} '{{}}' not found", id)))
        .map({pascal}Response::from)
}}

pub async fn create(
    conn: &mut AsyncPgConnection,
    user_id: Uuid,
    payload: Create{pascal}Request,
) -> Result<{pascal}Response, ApiError> {{
    let item = {pascal}Repository::create(conn, user_id, payload).await?;
    Ok({pascal}Response::from(item))
}}

pub async fn update(
    conn: &mut AsyncPgConnection,
    id: Uuid,
    user_id: Uuid,
    payload: Update{pascal}Request,
) -> Result<{pascal}Response, ApiError> {{
    let item = {pascal}Repository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{pascal} '{{}}' not found", id)))?;

    if item.user_id != user_id {{
        return Err(ApiError::Authorization(
            "You can only update your own {snake}s".to_string(),
        ));
    }}

    let updated = {pascal}Repository::update(conn, id, payload).await?;
    Ok({pascal}Response::from(updated))
}}

pub async fn delete(
    conn: &mut AsyncPgConnection,
    id: Uuid,
    user_id: Uuid,
) -> Result<(), ApiError> {{
    let item = {pascal}Repository::find_by_id(conn, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("{pascal} '{{}}' not found", id)))?;

    if item.user_id != user_id {{
        return Err(ApiError::Authorization(
            "You can only delete your own {snake}s".to_string(),
        ));
    }}

    {pascal}Repository::delete(conn, id).await?;
    Ok(())
}}
"#,
        snake = m.snake,
        pascal = m.pascal,
    )
}

fn template_handler(m: &ModuleNames) -> String {
    format!(
        r#"use axum::{{
    Extension, Json,
    extract::{{Path, State}},
    http::StatusCode,
}};
use uuid::Uuid;

use crate::bootstrap::models::AppState;
use crate::core::errors::{{ApiError, ErrorResponse}};
use crate::core::extractor::ValidatedJson;
use crate::modules::auth::helpers::Claims;
use crate::modules::{snake}::dto::{{Create{pascal}Request, {pascal}Response, Update{pascal}Request}};
use crate::modules::{snake}::service;

#[utoipa::path(
    get, path = "/api/{kebab}s", tag = "{snake}s",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = Vec<{pascal}Response>),
        (status = 401, body = ErrorResponse),
    )
)]
pub async fn get_all(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<{pascal}Response>>, ApiError> {{
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::get_all_by_user(&mut conn, claims.sub).await?))
}}

#[utoipa::path(
    get, path = "/api/{kebab}s/{{id}}", tag = "{snake}s",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID du {snake}")),
    responses(
        (status = 200, body = {pascal}Response),
        (status = 401, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    )
)]
pub async fn get_by_id(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<Json<{pascal}Response>, ApiError> {{
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::get_by_id(&mut conn, id).await?))
}}

#[utoipa::path(
    post, path = "/api/{kebab}s", tag = "{snake}s",
    security(("bearer_auth" = [])),
    request_body = Create{pascal}Request,
    responses(
        (status = 201, body = {pascal}Response),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    ValidatedJson(payload): ValidatedJson<Create{pascal}Request>,
) -> Result<(StatusCode, Json<{pascal}Response>), ApiError> {{
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    let item = service::create(&mut conn, claims.sub, payload).await?;
    Ok((StatusCode::CREATED, Json(item)))
}}

#[utoipa::path(
    put, path = "/api/{kebab}s/{{id}}", tag = "{snake}s",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID du {snake}")),
    request_body = Update{pascal}Request,
    responses(
        (status = 200, body = {pascal}Response),
        (status = 400, body = ErrorResponse),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    ValidatedJson(payload): ValidatedJson<Update{pascal}Request>,
) -> Result<Json<{pascal}Response>, ApiError> {{
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    Ok(Json(service::update(&mut conn, id, claims.sub, payload).await?))
}}

#[utoipa::path(
    delete, path = "/api/{kebab}s/{{id}}", tag = "{snake}s",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "UUID du {snake}")),
    responses(
        (status = 204, description = "{pascal} supprimé"),
        (status = 401, body = ErrorResponse),
        (status = 403, body = ErrorResponse),
        (status = 404, body = ErrorResponse),
    )
)]
pub async fn delete(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {{
    let mut conn = state.pool.get().await.map_err(ApiError::from)?;
    service::delete(&mut conn, id, claims.sub).await?;
    Ok(StatusCode::NO_CONTENT)
}}
"#,
        snake = m.snake,
        pascal = m.pascal,
        kebab = m.kebab,
    )
}

fn template_module_mod(m: &ModuleNames) -> String {
    format!(
        r#"use axum::{{Router, routing::{{delete, get, post, put}}}};
use crate::bootstrap::models::AppState;
use crate::core::middlewares::auth::require_auth;
use crate::core::middlewares::rate_limit::rate_limit_by_user;

pub mod dto;
pub mod handler;
pub mod service;

pub fn routes(state: AppState) -> Router {{
    Router::new()
        .route("/{kebab}s",      get(handler::get_all).post(handler::create))
        .route("/{kebab}s/{{id}}", get(handler::get_by_id))
        .route("/{kebab}s/{{id}}", put(handler::update))
        .route("/{kebab}s/{{id}}", delete(handler::delete))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            rate_limit_by_user,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ))
        .with_state(state)
}}
"#,
        kebab = m.kebab,
    )
}

fn template_migration_up(m: &ModuleNames) -> String {
    format!(
        r#"-- Migration : create_{snake}s
-- Générée par scaffold

CREATE TABLE IF NOT EXISTS {snake}s (
    id         UUID      PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID      NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- TODO: ajoute tes colonnes ici
    -- Exemple :
    -- title      VARCHAR(255) NOT NULL,
    -- content    TEXT         NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_{snake}s_user_id ON {snake}s(user_id);

CREATE TRIGGER set_updated_at
    BEFORE UPDATE ON {snake}s
    FOR EACH ROW
EXECUTE FUNCTION trigger_set_updated_at();
"#,
        snake = m.snake,
    )
}

fn template_migration_down(m: &ModuleNames) -> String {
    format!(
        r#"DROP TRIGGER IF EXISTS set_updated_at ON {snake}s;
DROP TABLE IF EXISTS {snake}s;
"#,
        snake = m.snake,
    )
}
