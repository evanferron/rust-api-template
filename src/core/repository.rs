//! # Repository générique
//!
//! Ce module fournit un ensemble de macros pour générer automatiquement
//! les méthodes courantes sur les repositories Diesel.
//!
//! ## Utilisation rapide
//!
//! ```rust
//! // Méthodes fondamentales — find_all, find_by_id, delete
//! impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);
//!
//! // Méthodes optionnelles à la carte
//! impl_exists!(PostRepository, crate::db::schema::posts, Uuid);
//! impl_count!(PostRepository, crate::db::schema::posts);
//! impl_find_paginated!(PostRepository, Post, crate::db::schema::posts, created_at);
//!
//! // Soft delete — nécessite une colonne deleted_at TIMESTAMP NULL
//! impl_soft_delete!(PostRepository, Post, crate::db::schema::posts, Uuid);
//! ```
//!
//! ## Tableau des macros
//!
//! | Macro | Méthodes générées | Prérequis |
//! |-------|-------------------|-----------|
//! | [`impl_base_repository!`] | `find_all`, `find_by_id`, `delete` | — |
//! | [`impl_find_all!`] | `find_all` | — |
//! | [`impl_find_by_id!`] | `find_by_id` | — |
//! | [`impl_delete!`] | `delete` | — |
//! | [`impl_exists!`] | `exists` | — |
//! | [`impl_count!`] | `count` | — |
//! | [`impl_find_paginated!`] | `find_paginated` | colonne de tri |
//! | [`impl_soft_delete!`] | `soft_delete`, `find_active`, `restore` | `deleted_at` |

use async_trait::async_trait;
use diesel_async::AsyncPgConnection;

use crate::core::errors::ApiError;

/// Macros disponibles pour les repositories.
pub use crate::{
    impl_base_repository, impl_count, impl_delete, impl_exists, impl_find_all, impl_find_by_id,
    impl_find_paginated, impl_soft_delete,
};

// ---------------------------------------------------------------------------
// Trait de base
// ---------------------------------------------------------------------------

#[async_trait]
pub trait BaseRepository<T, Id> {
    async fn find_all(conn: &mut AsyncPgConnection) -> Result<Vec<T>, ApiError>;
    async fn find_by_id(conn: &mut AsyncPgConnection, id: Id) -> Result<Option<T>, ApiError>;
    async fn delete(conn: &mut AsyncPgConnection, id: Id) -> Result<bool, ApiError>;
}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// Paramètres de pagination passés aux méthodes `find_paginated`.
#[derive(Debug, Clone)]
pub struct PaginationParams {
    /// Numéro de page (commence à 1)
    pub page: i64,
    /// Nombre d'éléments par page
    pub per_page: i64,
}

impl PaginationParams {
    pub fn new(page: i64, per_page: i64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, 100),
        }
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }
}

/// Réponse paginée avec métadonnées.
#[derive(Debug, serde::Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, params: &PaginationParams) -> Self {
        let total_pages = (total as f64 / params.per_page as f64).ceil() as i64;
        Self {
            data,
            total,
            page: params.page,
            per_page: params.per_page,
            total_pages,
        }
    }
}

// ---------------------------------------------------------------------------
// Macro composite — impl_base_repository!
// Sucre syntaxique qui appelle impl_find_all! + impl_find_by_id! + impl_delete!
// ---------------------------------------------------------------------------

/// Génère les 3 méthodes fondamentales : `find_all`, `find_by_id`, `delete`.
/// Équivalent d'appeler les 3 macros séparément.
///
/// # Exemple
/// ```rust
/// impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_base_repository {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => {
        $crate::impl_find_all!($repo, $model, $table);
        $crate::impl_find_by_id!($repo, $model, $table, $id_type);
        $crate::impl_delete!($repo, $table, $id_type);

        #[async_trait::async_trait]
        impl $crate::core::repository::BaseRepository<$model, $id_type> for $repo {
            async fn find_all(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<Vec<$model>, $crate::core::errors::ApiError> {
                <$repo>::find_all(conn).await
            }

            async fn find_by_id(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<Option<$model>, $crate::core::errors::ApiError> {
                <$repo>::find_by_id(conn, id).await
            }

            async fn delete(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::core::errors::ApiError> {
                <$repo>::delete(conn, id).await
            }
        }
    };
}

// ---------------------------------------------------------------------------
// impl_find_all!
// SELECT * FROM table
// ---------------------------------------------------------------------------

/// Génère la méthode `find_all` sur le repository.
///
/// # Exemple
/// ```rust
/// impl_find_all!(PostRepository, Post, crate::db::schema::posts);
/// ```
#[macro_export]
macro_rules! impl_find_all {
    ($repo:ty, $model:ty, $table:path) => {
        impl $repo {
            pub async fn find_all(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<Vec<$model>, $crate::core::errors::ApiError> {
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// impl_find_by_id!
// SELECT * FROM table WHERE id = ?
// ---------------------------------------------------------------------------

/// Génère la méthode `find_by_id` sur le repository.
///
/// # Exemple
/// ```rust
/// impl_find_by_id!(PostRepository, Post, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_find_by_id {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => {
        impl $repo {
            pub async fn find_by_id(
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
        }
    };
}

// ---------------------------------------------------------------------------
// impl_delete!
// DELETE FROM table WHERE id = ? → bool (true si une ligne supprimée)
// ---------------------------------------------------------------------------

/// Génère la méthode `delete` sur le repository.
///
/// # Exemple
/// ```rust
/// impl_delete!(PostRepository, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_delete {
    ($repo:ty, $table:path, $id_type:ty) => {
        impl $repo {
            pub async fn delete(
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

// ---------------------------------------------------------------------------
// impl_exists!
// SELECT EXISTS(SELECT 1 FROM table WHERE id = ?)
// ---------------------------------------------------------------------------

/// Génère la méthode `exists` sur le repository.
///
/// # Exemple
/// ```rust
/// impl_exists!(PostRepository, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_exists {
    ($repo:ty, $table:path, $id_type:ty) => {
        impl $repo {
            pub async fn exists(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::core::errors::ApiError> {
                use diesel::QueryDsl;
                use diesel::dsl::exists;
                use diesel::select;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                select(exists(_table::table.find(id)))
                    .get_result::<bool>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// impl_count!
// SELECT COUNT(*) FROM table
// ---------------------------------------------------------------------------

/// Génère la méthode `count` sur le repository.
///
/// # Exemple
/// ```rust
/// impl_count!(PostRepository, crate::db::schema::posts);
/// ```
#[macro_export]
macro_rules! impl_count {
    ($repo:ty, $table:path) => {
        impl $repo {
            pub async fn count(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<i64, $crate::core::errors::ApiError> {
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// impl_find_paginated!
// SELECT * FROM table ORDER BY $order_col DESC LIMIT per_page OFFSET offset
// ---------------------------------------------------------------------------

/// Génère la méthode `find_paginated` sur le repository.
/// Nécessite une colonne de tri explicite (typiquement `created_at`).
///
/// # Exemple
/// ```rust
/// impl_find_paginated!(PostRepository, Post, crate::db::schema::posts, created_at);
/// ```
#[macro_export]
macro_rules! impl_find_paginated {
    ($repo:ty, $model:ty, $table:path, $order_col:ident) => {
        impl $repo {
            pub async fn find_paginated(
                conn: &mut diesel_async::AsyncPgConnection,
                params: $crate::core::repository::PaginationParams,
            ) -> Result<
                $crate::core::repository::PaginatedResponse<$model>,
                $crate::core::errors::ApiError,
            > {
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                // COUNT pour les métadonnées de pagination
                let total = _table::table
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                // Page de données triée et découpée
                let data = _table::table
                    .order(_table::$order_col.desc())
                    .limit(params.per_page)
                    .offset(params.offset())
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                Ok($crate::core::repository::PaginatedResponse::new(
                    data, total, &params,
                ))
            }
        }
    };
}

// ---------------------------------------------------------------------------
// impl_soft_delete!
// UPDATE table SET deleted_at = NOW() WHERE id = ?
// Prérequis : colonne `deleted_at TIMESTAMP NULL` sur la table
// ---------------------------------------------------------------------------

/// Génère les méthodes `soft_delete` et `find_active` sur le repository.
///
/// `soft_delete` marque la ligne comme supprimée sans la retirer physiquement.
/// `find_active` retourne uniquement les lignes où `deleted_at IS NULL`.
///
/// Prérequis : la table doit avoir une colonne `deleted_at TIMESTAMP NULL`.
///
/// # Exemple
/// ```rust
/// impl_soft_delete!(PostRepository, Post, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_soft_delete {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => {
        impl $repo {
            /// Marque la ligne comme supprimée (deleted_at = NOW()).
            /// La ligne reste en base et peut être restaurée.
            pub async fn soft_delete(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::core::errors::ApiError> {
                use diesel::{ExpressionMethods, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                let rows = diesel::update(_table::table.find(id))
                    .set(_table::deleted_at.eq(Some(chrono::Utc::now().naive_utc())))
                    .execute(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                Ok(rows > 0)
            }

            /// Retourne uniquement les lignes non supprimées (deleted_at IS NULL).
            pub async fn find_active(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<Vec<$model>, $crate::core::errors::ApiError> {
                use diesel::{ExpressionMethods, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .filter(_table::deleted_at.is_null())
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)
            }

            /// Restaure une ligne soft-supprimée (deleted_at = NULL).
            pub async fn restore(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::core::errors::ApiError> {
                use diesel::{ExpressionMethods, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                let rows = diesel::update(_table::table.find(id))
                    .set(_table::deleted_at.eq(None::<chrono::NaiveDateTime>))
                    .execute(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                Ok(rows > 0)
            }
        }
    };
}
