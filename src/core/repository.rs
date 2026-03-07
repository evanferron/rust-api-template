use async_trait::async_trait;
use diesel_async::AsyncPgConnection;

use crate::core::errors::ApiError;

// ---------------------------------------------------------------------------
// Trait de base
// ---------------------------------------------------------------------------

/// Contrat minimal que tout repository doit respecter.
/// Implémenté automatiquement via `impl_base_repository!`.
///
/// `T`  — modèle Diesel (ex: User)
/// `Id` — type de la clé primaire (ex: Uuid, i32)
#[async_trait]
pub trait BaseRepository<T, Id> {
    async fn find_all(conn: &mut AsyncPgConnection) -> Result<Vec<T>, ApiError>;
    async fn find_by_id(conn: &mut AsyncPgConnection, id: Id) -> Result<Option<T>, ApiError>;
    async fn delete(conn: &mut AsyncPgConnection, id: Id) -> Result<bool, ApiError>;
}

// ---------------------------------------------------------------------------
// Macro
// ---------------------------------------------------------------------------

/// Génère l'implémentation de `BaseRepository` pour un repository donné.
///
/// # Arguments
/// - `$repo`     — struct du repository (ex: `UserRepository`)
/// - `$model`    — struct du modèle Diesel (ex: `User`)
/// - `$table`    — chemin vers le module de la table dans schema.rs (ex: `crate::db::schema::users`)
/// - `$id_type`  — type de la clé primaire (ex: `uuid::Uuid`, `i32`)
///
/// # Exemple
/// ```rust
/// impl_base_repository!(UserRepository, User, crate::db::schema::users, uuid::Uuid);
/// ```
///
/// Génère automatiquement :
/// - `find_all`    → SELECT * FROM table ORDER BY created_at DESC
/// - `find_by_id`  → SELECT * FROM table WHERE id = ?
/// - `delete`      → DELETE FROM table WHERE id = ? → bool
#[macro_export]
macro_rules! impl_base_repository {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => {
        #[async_trait::async_trait]
        impl $crate::core::repository::BaseRepository<$model, $id_type> for $repo {
            async fn find_all(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<Vec<$model>, $crate::core::errors::ApiError> {
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)
            }

            async fn find_by_id(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<Option<$model>, $crate::core::errors::ApiError> {
                use diesel::{OptionalExtension, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .find(id)
                    .first::<$model>(conn)
                    .await
                    .optional()
                    .map_err($crate::core::errors::ApiError::from)
            }

            async fn delete(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::core::errors::ApiError> {
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                let rows = diesel::delete(_table::table.find(id))
                    .execute(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                Ok(rows > 0)
            }
        }
    };
}
