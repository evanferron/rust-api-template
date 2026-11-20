//! État partagé par tous les scénarios Cucumber et helpers pour appeler l'API.

use std::collections::HashMap;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, header};
use cucumber::World;
use cucumber::gherkin::Step;
use serde_json::Value;
use tower::ServiceExt;

use crate::common::get_test_app;

#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct ApiWorld {
    /// Router complet (identique à la prod)
    app: Router,
    /// Statut HTTP de la dernière réponse
    pub status: u16,
    /// Corps JSON de la dernière réponse (`Null` si vide)
    pub body: Value,
    /// Dernier en-tête `Set-Cookie` reçu (refresh token)
    pub cookie: Option<String>,
    /// Access tokens par email
    tokens: HashMap<String, String>,
    /// Email de l'utilisateur courant
    current_user: Option<String>,
    /// Id de la dernière ressource créée (ex: post)
    pub last_id: Option<String>,
}

impl ApiWorld {
    async fn new() -> Self {
        Self {
            app: get_test_app().await.clone(),
            status: 0,
            body: Value::Null,
            cookie: None,
            tokens: HashMap::new(),
            current_user: None,
            last_id: None,
        }
    }

    // -----------------------------------------------------------------------
    // Requêtes
    // -----------------------------------------------------------------------

    /// Envoie une requête et enregistre statut, corps et cookie dans le world.
    /// Le cookie de refresh est joint automatiquement aux appels à `/auth/refresh`.
    pub async fn send(
        &mut self,
        method: Method,
        uri: &str,
        json: Option<Value>,
        token: Option<String>,
    ) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if uri.ends_with("/auth/refresh")
            && let Some(cookie) = &self.cookie
        {
            builder = builder.header(header::COOKIE, cookie);
        }
        let body = match json {
            Some(json) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };

        let response = self
            .app
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();

        self.status = response.status().as_u16();
        if let Some(cookie) = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
        {
            self.cookie = Some(cookie.to_string());
        }
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        self.body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    }

    /// Requête authentifiée avec le token de l'utilisateur courant.
    pub async fn send_as_current(&mut self, method: Method, uri: &str, json: Option<Value>) {
        let token = self.current_token();
        self.send(method, uri, json, Some(token)).await;
    }

    // -----------------------------------------------------------------------
    // Utilisateurs
    // -----------------------------------------------------------------------

    /// Inscrit l'utilisateur (si besoin), le connecte et en fait l'utilisateur courant.
    pub async fn login_as(&mut self, email: &str, password: &str) {
        let credentials = serde_json::json!({ "email": email, "password": password });
        let mut register = credentials.clone();
        register["first_name"] = "Test".into();
        register["last_name"] = "User".into();

        self.send(Method::POST, "/api/auth/register", Some(register), None)
            .await;
        self.send(Method::POST, "/api/auth/login", Some(credentials), None)
            .await;
        assert_eq!(self.status, 200, "login de {email} échoué : {}", self.body);

        let token = self.body["access_token"].as_str().unwrap().to_string();
        self.tokens.insert(email.to_string(), token);
        self.current_user = Some(email.to_string());
    }

    pub fn current_user(&self) -> String {
        self.current_user
            .clone()
            .expect("Aucun utilisateur courant : utiliser d'abord 'je suis connecté en tant que…'")
    }

    pub fn current_token(&self) -> String {
        self.token_of(&self.current_user())
    }

    pub fn token_of(&self, email: &str) -> String {
        self.tokens
            .get(email)
            .cloned()
            .unwrap_or_else(|| panic!("Aucun token pour {email}"))
    }

    /// Id d'un utilisateur connecté au cours du scénario (via `GET /api/users`, 100 premiers).
    /// Ne modifie pas le statut ni le corps de la dernière réponse.
    pub async fn user_id_of(&mut self, email: &str) -> String {
        let (status, body) = (self.status, self.body.clone());
        let token = self.token_of(email);
        self.send(Method::GET, "/api/users?per_page=100", None, Some(token))
            .await;
        let id = self.body["data"]
            .as_array()
            .and_then(|users| users.iter().find(|u| u["email"] == email))
            .and_then(|u| u["id"].as_str())
            .unwrap_or_else(|| panic!("Utilisateur {email} introuvable"))
            .to_string();
        (self.status, self.body) = (status, body);
        id
    }

    pub fn last_id(&self) -> String {
        self.last_id
            .clone()
            .expect("Aucune ressource créée dans ce scénario")
    }
}

// ---------------------------------------------------------------------------
// Tables Gherkin
// ---------------------------------------------------------------------------

/// Table à deux colonnes `| clé | valeur |` → map.
pub fn table(step: &Step) -> HashMap<String, String> {
    step.table()
        .expect("L'étape doit avoir une table de données")
        .rows
        .iter()
        .map(|row| (row[0].trim().to_string(), row[1].trim().to_string()))
        .collect()
}

/// Table à deux colonnes → objet JSON (`true` / `false` deviennent des booléens).
pub fn table_json(step: &Step) -> Value {
    table(step)
        .into_iter()
        .map(|(key, value)| {
            let value = match value.as_str() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => Value::String(value),
            };
            (key, value)
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
}
