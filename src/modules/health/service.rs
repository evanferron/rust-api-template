use std::sync::atomic::Ordering;
use std::time::Duration;

use diesel_async::RunQueryDsl;

use crate::common::error::{ApiError, ErrorCode};
use crate::common::state::AppState;

/// Durée maximale de la vérification de la base.
const DATABASE_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

/// Vérifie que le serveur peut recevoir du trafic.
/// Erreur `SERVICE_UNAVAILABLE` (503) si l'arrêt est en cours ou si la base ne répond pas.
pub async fn check_ready(state: &AppState) -> Result<(), ApiError> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(unavailable("Server is shutting down"));
    }

    let ping = async {
        let mut conn = state.pool.get().await?;
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .await
            .map_err(ApiError::internal)
    };

    match tokio::time::timeout(DATABASE_CHECK_TIMEOUT, ping).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(err)) => Err(unavailable("Database is unavailable").with_source(err)),
        Err(_) => Err(unavailable("Database check timed out")),
    }
}

fn unavailable(detail: &str) -> ApiError {
    ApiError::new(ErrorCode::ServiceUnavailable).with_detail(detail)
}
