//! Rate limiting (en mémoire, via `governor`).
//!
//! Deux niveaux :
//! - [`rate_limit`] : middleware global sur `/api`. Le client est identifié par son user id
//!   si un token valide est présent, sinon par son IP.
//! - [`AuthRateLimit`] : extracteur à ajouter aux handlers sensibles (login, register…),
//!   avec une limite stricte par IP contre le brute force.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{Extensions, HeaderMap};
use axum::middleware::Next;
use axum::response::Response;
use governor::clock::{Clock, DefaultClock};
use governor::{NotUntil, Quota, RateLimiter, state::keyed::DefaultKeyedStateStore};

use crate::common::error::{ApiError, ErrorCode};
use crate::common::jwt::{bearer_token, verify_token};
use crate::common::state::AppState;
use crate::config::RateLimitConfig;

type KeyedLimiter<K> = Arc<RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock>>;

/// Limiters partagés (clonables, thread-safe).
#[derive(Clone)]
pub struct RateLimitStore {
    /// Limite générale, par user id ou par IP.
    pub by_client: KeyedLimiter<String>,
    /// Limite stricte des routes d'authentification, par IP.
    pub auth: KeyedLimiter<IpAddr>,
}

impl RateLimitStore {
    pub fn new(config: &RateLimitConfig) -> Self {
        Self {
            by_client: Arc::new(RateLimiter::keyed(Quota::per_minute(
                config.requests_per_minute,
            ))),
            auth: Arc::new(RateLimiter::keyed(Quota::per_minute(
                config.auth_requests_per_minute,
            ))),
        }
    }

    /// Purge les clés inactives (à appeler périodiquement).
    pub fn retain_recent(&self) {
        self.by_client.retain_recent();
        self.auth.retain_recent();
    }
}

/// Middleware global : identifie le client par son user id (token valide) ou son IP.
pub async fn rate_limit(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let key = match bearer_token(req.headers())
        .and_then(|token| verify_token(token, &state.config.jwt.secret).ok())
    {
        Some(claims) => format!("user:{}", claims.sub),
        None => format!("ip:{}", client_ip(&state, req.headers(), req.extensions())),
    };

    state
        .rate_limit
        .by_client
        .check_key(&key)
        .map_err(|not_until| {
            tracing::warn!(client = %key, "Rate limit exceeded");
            rate_limited(not_until)
        })?;

    Ok(next.run(req).await)
}

/// Extracteur de limite stricte par IP pour les routes d'authentification.
///
/// ```rust,ignore
/// pub async fn login(_: AuthRateLimit, State(state): State<AppState>, ...) -> ...
/// ```
pub struct AuthRateLimit;

impl FromRequestParts<AppState> for AuthRateLimit {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        let ip = client_ip(state, &parts.headers, &parts.extensions);
        state.rate_limit.auth.check_key(&ip).map_err(|not_until| {
            tracing::warn!(ip = %ip, "Auth rate limit exceeded");
            rate_limited(not_until)
        })?;
        Ok(AuthRateLimit)
    }
}

/// IP du client.
///
/// - Par défaut : IP de la connexion TCP.
/// - Avec `TRUST_PROXY=true` : dernière IP de `X-Forwarded-For`, c'est-à-dire celle ajoutée
///   par le reverse proxy (les entrées précédentes sont fournies par le client et falsifiables).
///   À n'activer que si l'API n'est joignable **que** via ce proxy.
fn client_ip(state: &AppState, headers: &HeaderMap, extensions: &Extensions) -> IpAddr {
    let forwarded = state
        .config
        .http
        .trust_proxy
        .then(|| forwarded_ip(headers))
        .flatten();

    forwarded.unwrap_or_else(|| {
        extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    })
}

fn forwarded_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .last()?
        .trim()
        .parse()
        .ok()
}

/// Erreur `RATE_LIMITED` avec l'en-tête `Retry-After` (secondes, arrondi au supérieur).
fn rate_limited(not_until: NotUntil<<DefaultClock as Clock>::Instant>) -> ApiError {
    let wait = not_until.wait_time_from(DefaultClock::default().now());
    let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    ApiError::new(ErrorCode::RateLimited).with_retry_after(seconds.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_ip_takes_the_entry_added_by_the_proxy() {
        let mut headers = HeaderMap::new();
        assert_eq!(forwarded_ip(&headers), None);
        headers.insert("x-forwarded-for", "1.1.1.1, 2.2.2.2".parse().unwrap());
        assert_eq!(forwarded_ip(&headers), Some("2.2.2.2".parse().unwrap()));
        headers.insert("x-forwarded-for", "garbage".parse().unwrap());
        assert_eq!(forwarded_ip(&headers), None);
    }
}
