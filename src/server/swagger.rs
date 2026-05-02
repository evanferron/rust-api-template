use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

use crate::core::errors::ErrorResponse;
use crate::modules::auth::dto::{LoginRequest, LoginResponse, RefreshResponse, RegisterRequest};
use crate::modules::health::dto::HealthResponse;
use crate::modules::user::dto::{UpdateUserRequest, UserResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::modules::health::handler::health_check,
        crate::modules::auth::handler::register,
        crate::modules::auth::handler::login,
        crate::modules::auth::handler::refresh,
        crate::modules::user::handler::get_all,
        crate::modules::user::handler::get_by_id,
        crate::modules::user::handler::update,
        crate::modules::user::handler::delete,
        crate::modules::auth::handler::logout
    ),
    components(
        schemas(
            ErrorResponse,
            HealthResponse,
            RegisterRequest,
            LoginRequest,
            LoginResponse,
            RefreshResponse,
            UserResponse,
            UpdateUserRequest,
        )
    ),
    tags(
        (name = "health", description = "Vérification de l'état du serveur"),
        (name = "auth",   description = "Authentification et gestion des tokens"),
        (name = "users",  description = "Gestion des utilisateurs"),
    ),
    info(
        title = "API Template",
        version = env!("CARGO_PKG_VERSION"),
        description = "API Template Rust — Axum + Diesel + PostgreSQL",
        contact(
            name = "Evan",
            email = "evan.ferron53@gmail.com"
        ),
    ),
    modifiers(&SecurityAddon),
)]
pub struct ApiDoc;

/// Ajoute le schéma Bearer JWT à la doc Swagger.
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

// ─── Tests unitaires pour la documentation OpenAPI ─────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::OpenApi;

    // ─── Helper ─────────────────────────────────────────────────────────────

    fn build_doc() -> utoipa::openapi::OpenApi {
        ApiDoc::openapi()
    }

    // ─── Info ────────────────────────────────────────────────────────────────

    #[test]
    fn test_openapi_title() {
        let doc = build_doc();
        assert_eq!(doc.info.title, "API Template");
    }

    #[test]
    fn test_openapi_version_matches_cargo() {
        let doc = build_doc();
        assert_eq!(doc.info.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_openapi_contact_email() {
        let doc = build_doc();
        let contact = doc.info.contact.expect("contact should be set");
        assert_eq!(contact.email.as_deref(), Some("evan.ferron53@gmail.com"));
    }

    #[test]
    fn test_openapi_contact_name() {
        let doc = build_doc();
        let contact = doc.info.contact.expect("contact should be set");
        assert_eq!(contact.name.as_deref(), Some("Evan"));
    }

    // ─── Tags ────────────────────────────────────────────────────────────────

    #[test]
    fn test_openapi_tags_present() {
        let doc = build_doc();
        let tags: Vec<&str> = doc
            .tags
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|t| t.name.as_str())
            .collect();

        assert!(tags.contains(&"health"), "tag 'health' manquant");
        assert!(tags.contains(&"auth"), "tag 'auth' manquant");
        assert!(tags.contains(&"users"), "tag 'users' manquant");
    }

    #[test]
    fn test_openapi_tag_count() {
        let doc = build_doc();
        let count = doc.tags.as_deref().unwrap_or_default().len();
        assert_eq!(count, 3);
    }

    // ─── Paths ───────────────────────────────────────────────────────────────

    #[test]
    fn test_openapi_health_path_exists() {
        let doc = build_doc();
        assert!(
            doc.paths.paths.contains_key("/api/health"),
            "/api/health manquant"
        );
    }

    #[test]
    fn test_openapi_auth_paths_exist() {
        let doc = build_doc();
        let paths = &doc.paths.paths;
        assert!(
            paths.contains_key("/api/auth/register"),
            "/api/auth/register manquant"
        );
        assert!(
            paths.contains_key("/api/auth/login"),
            "/api/auth/login manquant"
        );
        assert!(
            paths.contains_key("/api/auth/refresh"),
            "/api/auth/refresh manquant"
        );
    }

    // ─── Schemas ─────────────────────────────────────────────────────────────

    #[test]
    fn test_openapi_schemas_present() {
        let doc = build_doc();
        let schemas = &doc
            .components
            .as_ref()
            .expect("components should be set")
            .schemas;

        let expected = [
            "ErrorResponse",
            "HealthResponse",
            "RegisterRequest",
            "LoginRequest",
            "LoginResponse",
            "RefreshResponse",
            "UserResponse",
            "UpdateUserRequest",
        ];

        for name in &expected {
            assert!(schemas.contains_key(*name), "schema '{}' manquant", name);
        }
    }

    // ─── SecurityAddon ───────────────────────────────────────────────────────

    #[test]
    fn test_security_addon_adds_bearer_scheme() {
        let mut doc = utoipa::openapi::OpenApiBuilder::new()
            .info(utoipa::openapi::InfoBuilder::new().title("test").build())
            .build();

        SecurityAddon.modify(&mut doc);

        let schemes = &doc
            .components
            .as_ref()
            .expect("components should exist after modify")
            .security_schemes;

        assert!(
            schemes.contains_key("bearer_auth"),
            "bearer_auth scheme manquant"
        );
    }

    #[test]
    fn test_security_addon_scheme_is_http_bearer() {
        use utoipa::openapi::security::SecurityScheme;

        let mut doc = utoipa::openapi::OpenApiBuilder::new()
            .info(utoipa::openapi::InfoBuilder::new().title("test").build())
            .build();

        SecurityAddon.modify(&mut doc);

        let scheme = doc
            .components
            .as_ref()
            .unwrap()
            .security_schemes
            .get("bearer_auth")
            .unwrap();

        assert!(
            matches!(scheme, SecurityScheme::Http(_)),
            "le schéma devrait être Http"
        );
    }

    #[test]
    fn test_security_addon_idempotent() {
        // Appeler modify deux fois ne doit pas dupliquer ni écraser le schéma
        let mut doc = utoipa::openapi::OpenApiBuilder::new()
            .info(utoipa::openapi::InfoBuilder::new().title("test").build())
            .build();

        SecurityAddon.modify(&mut doc);
        SecurityAddon.modify(&mut doc);

        let count = doc.components.as_ref().unwrap().security_schemes.len();

        assert_eq!(count, 1, "modify deux fois ne doit pas créer de doublon");
    }

    #[test]
    fn test_openapi_doc_is_serializable() {
        // Vérifie que le doc entier peut être sérialisé en JSON valide
        let doc = build_doc();
        let json = serde_json::to_string(&doc);
        assert!(
            json.is_ok(),
            "le doc OpenAPI doit être sérialisable en JSON"
        );
    }
}
