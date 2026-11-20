//! Hachage et vérification des mots de passe (bcrypt).
//!
//! bcrypt est coûteux en CPU : il s'exécute dans un thread bloquant pour ne pas bloquer le runtime.
//! Coût réduit (4) en build debug pour accélérer les tests, 12 en release.

use crate::common::error::ApiError;

/// Hache un mot de passe.
pub async fn hash_password(password: &str) -> Result<String, ApiError> {
    let password = password.to_string();
    let cost = if cfg!(debug_assertions) { 4 } else { 12 };
    tokio::task::spawn_blocking(move || bcrypt::hash(&password, cost))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::internal)
}

/// Retourne `true` si `password` correspond au `hash`.
/// L'appelant choisit l'erreur à renvoyer (`INVALID_CREDENTIALS`, `INVALID_CURRENT_PASSWORD`…).
pub async fn verify_password(password: &str, hash: &str) -> Result<bool, ApiError> {
    let password = password.to_string();
    let hash = hash.to_string();
    tokio::task::spawn_blocking(move || bcrypt::verify(&password, &hash))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::internal)
}
