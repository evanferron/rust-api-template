//! # Repository générique
//!
//! Chaque capacité générique est un **trait** distinct, implémenté par une **macro** dédiée.
//! Un repository n'implémente que les capacités dont il a besoin : aucune méthode
//! n'est générée « dans le vide ».
//!
//! ## Utilisation
//!
//! ```rust,ignore
//! use crate::core::repository::prelude::*;
//!
//! pub struct PostRepository;
//!
//! // Socle CRUD (FindAll + FindById + Delete) → BaseRepository automatiquement
//! crate::impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);
//!
//! // Capacités à la carte
//! crate::impl_paginate!(PostRepository, Post, crate::db::schema::posts, created_at);
//! crate::impl_count!(PostRepository, crate::db::schema::posts);
//!
//! // Appel (le trait doit être dans le scope → importer le prelude)
//! let post = PostRepository::find_by_id(conn, id).await?;
//! ```
//!
//! ## Capacités disponibles
//!
//! | Trait | Macro | Méthodes | Prérequis |
//! |-------|-------|----------|-----------|
//! | [`FindAll`] | [`impl_find_all!`] | `find_all` | — |
//! | [`FindById`] | [`impl_find_by_id!`] | `find_by_id` | — |
//! | [`Delete`] | [`impl_delete!`] | `delete` | — |
//! | [`Exists`] | [`impl_exists!`] | `exists` | — |
//! | [`Count`] | [`impl_count!`] | `count` | — |
//! | [`Paginate`] | [`impl_paginate!`] | `find_paginated`, `find_paginated_by` | colonne de tri, clé primaire simple |
//! | [`SoftDelete`] | [`impl_soft_delete!`] | `soft_delete`, `find_active`, `restore` | colonne `deleted_at TIMESTAMP NULL` |
//! | [`BaseRepository`] | [`impl_base_repository!`] | `FindAll` + `FindById` + `Delete` | — |
//!
//! `BaseRepository` est un supertrait implémenté automatiquement pour tout type
//! qui implémente `FindAll`, `FindById` et `Delete`.
//!
//! Les méthodes spécifiques à une entité (recherche par email, jointures…) s'écrivent
//! dans un bloc `impl` classique à côté des macros.

use std::future::Future;

use diesel::expression::BoxableExpression;
use diesel::pg::Pg;
use diesel::sql_types::Bool;
use diesel_async::AsyncPgConnection;
use serde::Serialize;
use utoipa::ToSchema;

use crate::core::errors::ApiError;
use crate::core::params::PaginationQuery;

/// Macros disponibles pour les repositories.
pub use crate::{
    impl_base_repository, impl_count, impl_delete, impl_exists, impl_find_all, impl_find_by_id,
    impl_paginate, impl_soft_delete,
};

/// Imports nécessaires pour appeler les méthodes génériques d'un repository.
///
/// ```rust,ignore
/// use crate::core::repository::prelude::*;
/// ```
pub mod prelude {
    pub use super::{
        BaseRepository, Count, Delete, Exists, Filter, FindAll, FindById, Paginate,
        PaginatedResponse, PaginationParams, SoftDelete,
    };
}

// ---------------------------------------------------------------------------
// Traits — une capacité par trait
// ---------------------------------------------------------------------------

/// `SELECT * FROM table`
pub trait FindAll<M> {
    fn find_all(
        conn: &mut AsyncPgConnection,
    ) -> impl Future<Output = Result<Vec<M>, ApiError>> + Send;
}

/// `SELECT * FROM table WHERE pk = ?` → `None` si absent.
pub trait FindById<M, Id> {
    fn find_by_id(
        conn: &mut AsyncPgConnection,
        id: Id,
    ) -> impl Future<Output = Result<Option<M>, ApiError>> + Send;
}

/// `DELETE FROM table WHERE pk = ?` → `true` si une ligne a été supprimée.
pub trait Delete<Id> {
    fn delete(
        conn: &mut AsyncPgConnection,
        id: Id,
    ) -> impl Future<Output = Result<bool, ApiError>> + Send;
}

/// `SELECT EXISTS(SELECT 1 FROM table WHERE pk = ?)`
pub trait Exists<Id> {
    fn exists(
        conn: &mut AsyncPgConnection,
        id: Id,
    ) -> impl Future<Output = Result<bool, ApiError>> + Send;
}

/// `SELECT COUNT(*) FROM table`
pub trait Count {
    fn count(conn: &mut AsyncPgConnection) -> impl Future<Output = Result<i64, ApiError>> + Send;
}

/// Filtre dynamique applicable à une table, utilisé par [`Paginate::find_paginated_by`].
///
/// ```rust,ignore
/// let filter = || -> Filter<posts::table> { Box::new(posts::user_id.eq(user_id)) };
/// ```
pub type Filter<T> = Box<dyn BoxableExpression<T, Pg, SqlType = Bool>>;

/// Pagination triée par ordre décroissant sur une colonne (puis sur la clé primaire
/// pour garantir un ordre stable entre les pages).
pub trait Paginate<M> {
    /// Table Diesel ciblée (ex: `posts::table`).
    type Table;

    /// Page sur toute la table.
    fn find_paginated(
        conn: &mut AsyncPgConnection,
        params: PaginationParams,
    ) -> impl Future<Output = Result<PaginatedResponse<M>, ApiError>> + Send;

    /// Page filtrée. `filter` est appelé deux fois (COUNT puis SELECT),
    /// c'est pourquoi il s'agit d'une closure et non d'une expression.
    fn find_paginated_by<F>(
        conn: &mut AsyncPgConnection,
        params: PaginationParams,
        filter: F,
    ) -> impl Future<Output = Result<PaginatedResponse<M>, ApiError>> + Send
    where
        F: Fn() -> Filter<Self::Table> + Send;
}

/// Suppression logique via une colonne `deleted_at TIMESTAMP NULL`.
pub trait SoftDelete<M, Id> {
    /// `deleted_at = NOW()` → `true` si une ligne a été modifiée.
    fn soft_delete(
        conn: &mut AsyncPgConnection,
        id: Id,
    ) -> impl Future<Output = Result<bool, ApiError>> + Send;

    /// Lignes où `deleted_at IS NULL`.
    fn find_active(
        conn: &mut AsyncPgConnection,
    ) -> impl Future<Output = Result<Vec<M>, ApiError>> + Send;

    /// `deleted_at = NULL` → `true` si une ligne a été modifiée.
    fn restore(
        conn: &mut AsyncPgConnection,
        id: Id,
    ) -> impl Future<Output = Result<bool, ApiError>> + Send;
}

/// Socle CRUD : implémenté automatiquement pour tout repository qui implémente
/// [`FindAll`], [`FindById`] et [`Delete`] (voir [`impl_base_repository!`]).
///
/// Utile comme borne générique : `fn foo<R: BaseRepository<M, Id>>()`.
pub trait BaseRepository<M, Id>: FindAll<M> + FindById<M, Id> + Delete<Id> {}

impl<R, M, Id> BaseRepository<M, Id> for R where R: FindAll<M> + FindById<M, Id> + Delete<Id> {}

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// Paramètres de pagination normalisés (page ≥ 1, 1 ≤ per_page ≤ 100).
#[derive(Debug, Clone, Copy)]
pub struct PaginationParams {
    /// Numéro de page (commence à 1)
    pub page: i64,
    /// Nombre d'éléments par page
    pub per_page: i64,
}

impl PaginationParams {
    pub const DEFAULT_PAGE: i64 = 1;
    pub const DEFAULT_PER_PAGE: i64 = 20;
    pub const MAX_PER_PAGE: i64 = 100;

    pub fn new(page: i64, per_page: i64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, Self::MAX_PER_PAGE),
        }
    }

    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.per_page
    }
}

impl From<PaginationQuery> for PaginationParams {
    fn from(query: PaginationQuery) -> Self {
        Self::new(
            query.page.unwrap_or(Self::DEFAULT_PAGE),
            query.per_page.unwrap_or(Self::DEFAULT_PER_PAGE),
        )
    }
}

/// Réponse paginée avec métadonnées.
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, params: &PaginationParams) -> Self {
        Self {
            data,
            total,
            page: params.page,
            per_page: params.per_page,
            total_pages: (total + params.per_page - 1) / params.per_page,
        }
    }

    /// Convertit les éléments (ex: modèle Diesel → DTO) en conservant les métadonnées.
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> PaginatedResponse<U> {
        PaginatedResponse {
            data: self.data.into_iter().map(f).collect(),
            total: self.total,
            page: self.page,
            per_page: self.per_page,
            total_pages: self.total_pages,
        }
    }
}

// ---------------------------------------------------------------------------
// Macros
// ---------------------------------------------------------------------------

/// Implémente [`FindAll`], [`FindById`] et [`Delete`] — donc [`BaseRepository`].
///
/// ```rust,ignore
/// impl_base_repository!(PostRepository, Post, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_base_repository {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => {
        $crate::impl_find_all!($repo, $model, $table);
        $crate::impl_find_by_id!($repo, $model, $table, $id_type);
        $crate::impl_delete!($repo, $table, $id_type);
    };
}

/// Implémente [`FindAll`].
///
/// ```rust,ignore
/// impl_find_all!(PostRepository, Post, crate::db::schema::posts);
/// ```
#[macro_export]
macro_rules! impl_find_all {
    ($repo:ty, $model:ty, $table:path) => {
        impl $crate::core::repository::FindAll<$model> for $repo {
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
        }
    };
}

/// Implémente [`FindById`].
///
/// ```rust,ignore
/// impl_find_by_id!(PostRepository, Post, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_find_by_id {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => {
        impl $crate::core::repository::FindById<$model, $id_type> for $repo {
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
        }
    };
}

/// Implémente [`Delete`].
///
/// ```rust,ignore
/// impl_delete!(PostRepository, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_delete {
    ($repo:ty, $table:path, $id_type:ty) => {
        impl $crate::core::repository::Delete<$id_type> for $repo {
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

/// Implémente [`Exists`].
///
/// ```rust,ignore
/// impl_exists!(PostRepository, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_exists {
    ($repo:ty, $table:path, $id_type:ty) => {
        impl $crate::core::repository::Exists<$id_type> for $repo {
            async fn exists(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::core::errors::ApiError> {
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                diesel::select(diesel::dsl::exists(_table::table.find(id)))
                    .get_result::<bool>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)
            }
        }
    };
}

/// Implémente [`Count`].
///
/// ```rust,ignore
/// impl_count!(PostRepository, crate::db::schema::posts);
/// ```
#[macro_export]
macro_rules! impl_count {
    ($repo:ty, $table:path) => {
        impl $crate::core::repository::Count for $repo {
            async fn count(
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

/// Implémente [`Paginate`]. Tri décroissant sur `$order_col`, puis sur la clé primaire.
///
/// ```rust,ignore
/// impl_paginate!(PostRepository, Post, crate::db::schema::posts, created_at);
///
/// // Appel filtré
/// PostRepository::find_paginated_by(conn, params, || Box::new(posts::user_id.eq(user_id))).await?;
/// ```
#[macro_export]
macro_rules! impl_paginate {
    ($repo:ty, $model:ty, $($table:ident)::+, $order_col:ident) => {
        impl $crate::core::repository::Paginate<$model> for $repo {
            type Table = $($table)::+::table;

            async fn find_paginated(
                conn: &mut diesel_async::AsyncPgConnection,
                params: $crate::core::repository::PaginationParams,
            ) -> Result<
                $crate::core::repository::PaginatedResponse<$model>,
                $crate::core::errors::ApiError,
            > {
                use diesel::{ExpressionMethods, QueryDsl, Table};
                use diesel_async::RunQueryDsl;
                use $($table)::+ as _table;

                let total = _table::table
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                let data = _table::table
                    .order(_table::$order_col.desc())
                    .then_order_by(_table::table.primary_key().desc())
                    .limit(params.per_page)
                    .offset(params.offset())
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                Ok($crate::core::repository::PaginatedResponse::new(
                    data, total, &params,
                ))
            }

            async fn find_paginated_by<F>(
                conn: &mut diesel_async::AsyncPgConnection,
                params: $crate::core::repository::PaginationParams,
                filter: F,
            ) -> Result<
                $crate::core::repository::PaginatedResponse<$model>,
                $crate::core::errors::ApiError,
            >
            where
                F: Fn() -> $crate::core::repository::Filter<Self::Table> + Send,
            {
                use diesel::{ExpressionMethods, QueryDsl, Table};
                use diesel_async::RunQueryDsl;
                use $($table)::+ as _table;

                let total = _table::table
                    .filter(filter())
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                let data = _table::table
                    .filter(filter())
                    .order(_table::$order_col.desc())
                    .then_order_by(_table::table.primary_key().desc())
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

/// Implémente [`SoftDelete`]. La table doit avoir une colonne `deleted_at TIMESTAMP NULL`.
///
/// ```rust,ignore
/// impl_soft_delete!(PostRepository, Post, crate::db::schema::posts, Uuid);
/// ```
#[macro_export]
macro_rules! impl_soft_delete {
    ($repo:ty, $model:ty, $table:path, $id_type:ty) => {
        impl $crate::core::repository::SoftDelete<$model, $id_type> for $repo {
            async fn soft_delete(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::core::errors::ApiError> {
                use diesel::{ExpressionMethods, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                let rows = diesel::update(_table::table.find(id))
                    .set(_table::deleted_at.eq(diesel::dsl::now.nullable()))
                    .execute(conn)
                    .await
                    .map_err($crate::core::errors::ApiError::from)?;

                Ok(rows > 0)
            }

            async fn find_active(
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

            async fn restore(
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

// ---------------------------------------------------------------------------
// Tests unitaires — pagination (les macros sont testées dans tests/repository.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_params_are_clamped() {
        let p = PaginationParams::new(0, 500);
        assert_eq!((p.page, p.per_page), (1, PaginationParams::MAX_PER_PAGE));

        let p = PaginationParams::new(-5, -10);
        assert_eq!((p.page, p.per_page), (1, 1));

        let p = PaginationParams::new(3, 20);
        assert_eq!((p.page, p.per_page), (3, 20));
    }

    #[test]
    fn pagination_offset() {
        assert_eq!(PaginationParams::new(1, 20).offset(), 0);
        assert_eq!(PaginationParams::new(3, 10).offset(), 20);
    }

    #[test]
    fn pagination_from_query_uses_defaults() {
        let p = PaginationParams::from(PaginationQuery {
            page: None,
            per_page: None,
        });
        assert_eq!(p.page, PaginationParams::DEFAULT_PAGE);
        assert_eq!(p.per_page, PaginationParams::DEFAULT_PER_PAGE);
    }

    #[test]
    fn paginated_response_total_pages() {
        let p = PaginationParams::new(1, 10);
        assert_eq!(
            PaginatedResponse::<i32>::new(vec![], 100, &p).total_pages,
            10
        );
        assert_eq!(
            PaginatedResponse::<i32>::new(vec![], 101, &p).total_pages,
            11
        );
        assert_eq!(PaginatedResponse::<i32>::new(vec![], 3, &p).total_pages, 1);
        assert_eq!(PaginatedResponse::<i32>::new(vec![], 0, &p).total_pages, 0);
    }

    #[test]
    fn paginated_response_map_keeps_metadata() {
        let p = PaginationParams::new(2, 2);
        let mapped = PaginatedResponse::new(vec![1, 2], 5, &p).map(|n| n.to_string());
        assert_eq!(mapped.data, vec!["1", "2"]);
        assert_eq!(
            (
                mapped.total,
                mapped.page,
                mapped.per_page,
                mapped.total_pages
            ),
            (5, 2, 2, 3)
        );
    }
}
