//! Rate limiting (en mémoire, via `governor`).
//!
//! Deux niveaux :
//! - [`rate_limit`] : middleware global sur `/api`. Le client est identifié par son user id
//!   si un token valide est présent, sinon par son IP.
//! - [`AuthRateLimit`] : extracteur à ajouter aux handlers sensibles (login, register…),
//!   avec une limite stricte par IP contre le brute force.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::{ConnectInfo, FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::Response;
use governor::clock::{Clock, DefaultClock};
use governor::{NotUntil, Quota, RateLimiter, state::keyed::DefaultKeyedStateStore};

use crate::common::error::{ApiError, ErrorCode};
use crate::common::jwt::{bearer_token, verify_token};
use crate::common::state::AppState;

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
    /// `auth_rpm` : requêtes/minute sur les routes d'authentification (par IP).
    /// `default_rpm` : requêtes/minute sur toute l'API (par user ou IP).
    pub fn new(auth_rpm: u32, default_rpm: u32) -> Self {
        Self {
            by_client: Arc::new(RateLimiter::keyed(per_minute(default_rpm))),
            auth: Arc::new(RateLimiter::keyed(per_minute(auth_rpm))),
        }
    }

    /// Purge les clés inactives (à appeler périodiquement).
    pub fn retain_recent(&self) {
        self.by_client.retain_recent();
        self.auth.retain_recent();
    }
}

fn per_minute(rpm: u32) -> Quota {
    Quota::per_minute(NonZeroU32::new(rpm).expect("rate limit must be > 0"))
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
        None => format!("ip:{}", client_ip(req.extensions())),
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
        let ip = client_ip(&parts.extensions);
        state.rate_limit.auth.check_key(&ip).map_err(|not_until| {
            tracing::warn!(ip = %ip, "Auth rate limit exceeded");
            rate_limited(not_until)
        })?;
        Ok(AuthRateLimit)
    }
}

/// IP de la connexion TCP (attention : derrière un reverse proxy, c'est l'IP du proxy).
fn client_ip(extensions: &axum::http::Extensions) -> IpAddr {
    extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

/// Erreur `RATE_LIMITED` avec l'en-tête `Retry-After` (secondes, arrondi au supérieur).
fn rate_limited(not_until: NotUntil<<DefaultClock as Clock>::Instant>) -> ApiError {
    let wait = not_until.wait_time_from(DefaultClock::default().now());
    let seconds = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    ApiError::new(ErrorCode::RateLimited).with_retry_after(seconds.max(1))
}
