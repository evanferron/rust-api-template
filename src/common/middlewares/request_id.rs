//! Identifiant de requête (`x-request-id`).
//!
//! Réutilise l'en-tête entrant s'il est valide (ex: posé par un reverse proxy),
//! sinon génère un UUID. L'identifiant est :
//! - renvoyé dans l'en-tête de réponse `x-request-id`,
//! - ajouté au span de log de la requête,
//! - inclus dans le corps des réponses d'erreur (`request_id`).

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

pub const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

tokio::task_local! {
    static REQUEST_ID: String;
}

/// Identifiant de la requête en cours, si appelé dans le contexte d'une requête.
pub fn current() -> Option<String> {
    REQUEST_ID.try_with(Clone::clone).ok()
}

/// Middleware à placer en couche la plus externe du router.
pub async fn request_id(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(&X_REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .filter(|v| is_valid(v))
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let header = HeaderValue::from_str(&id).expect("request id is a valid header value");
    req.headers_mut().insert(X_REQUEST_ID, header.clone());

    let mut response = REQUEST_ID.scope(id, next.run(req)).await;
    response.headers_mut().insert(X_REQUEST_ID, header);
    response
}

/// Accepte uniquement des identifiants courts et sûrs (alphanumérique, `-`, `_`, `.`).
fn is_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_incoming_ids() {
        assert!(is_valid("4f1c2a7e-0b3d-4c55-9a8e-1f2b3c4d5e6f"));
        assert!(is_valid("req_123.abc"));
        assert!(!is_valid(""));
        assert!(!is_valid("bad id"));
        assert!(!is_valid(&"a".repeat(129)));
    }

    #[tokio::test]
    async fn current_is_scoped_to_the_request() {
        assert_eq!(current(), None);
        let inside = REQUEST_ID
            .scope("abc".to_string(), async { current() })
            .await;
        assert_eq!(inside.as_deref(), Some("abc"));
    }
}
