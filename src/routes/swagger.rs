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
