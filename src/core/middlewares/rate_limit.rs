use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use std::net::SocketAddr;

use crate::config::state::AppState;
use crate::core::errors::ApiError;
use crate::modules::auth::helpers::Claims;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type KeyedLimiter<K> = Arc<RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock>>;

/// Store partagé des rate limiters — un par "profil" de limite.
/// Clonable car wrappé dans Arc, safe à partager entre threads.
#[derive(Clone)]
pub struct RateLimitStore {
    /// Limiter par IP — pour les routes publiques
    pub by_ip: KeyedLimiter<IpAddr>,
    /// Limiter par user ID — pour les routes protégées
    pub by_user: KeyedLimiter<String>,
}

impl RateLimitStore {
    /// `auth_rpm`    — requêtes par minute sur les routes auth (login, register)
    /// `default_rpm` — requêtes par minute sur les routes protégées
    pub fn new(auth_rpm: u32, default_rpm: u32) -> Self {
        Self {
            by_ip: Arc::new(RateLimiter::keyed(Quota::per_minute(
                NonZeroU32::new(auth_rpm).unwrap(),
            ))),
            by_user: Arc::new(RateLimiter::keyed(Quota::per_minute(
                NonZeroU32::new(default_rpm).unwrap(),
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Middleware — routes publiques (par IP)
// ---------------------------------------------------------------------------

/// Rate limiter pour les routes auth (login, register, refresh).
/// Identifie le client par son IP.
/// Recommandé : 10-20 requêtes/minute pour les routes d'auth.
pub async fn rate_limit_by_ip(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0.ip())
        .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    state.rate_limit.by_ip.check_key(&ip).map_err(|_| {
        tracing::warn!(ip = %ip, "Rate limit exceeded (IP)");
        ApiError::RateLimitExceeded {
            client_id: ip.to_string(),
            max_requests: 0, // valeur symbolique, la vraie est dans le Quota
            window_duration: Duration::from_secs(60),
        }
    })?;

    Ok(next.run(req).await)
}

// ---------------------------------------------------------------------------
// Middleware — routes protégées (par user)
// ---------------------------------------------------------------------------

/// Rate limiter pour les routes protégées.
/// Identifie le client par son UUID utilisateur depuis les claims JWT.
/// Doit être placé APRÈS `require_auth` dans la chaîne de middleware.
pub async fn rate_limit_by_user(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Les claims sont injectés par require_auth avant ce middleware
    let user_id = req
        .extensions()
        .get::<Claims>()
        .map(|c| c.sub.to_string())
        .ok_or_else(|| ApiError::Authentication("Missing claims".to_string()))?;

    state.rate_limit.by_user.check_key(&user_id).map_err(|_| {
        tracing::warn!(user_id = %user_id, "Rate limit exceeded (user)");
        ApiError::RateLimitExceeded {
            client_id: user_id.clone(),
            max_requests: 0,
            window_duration: Duration::from_secs(60),
        }
    })?;

    Ok(next.run(req).await)
}
