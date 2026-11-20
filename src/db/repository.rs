//! # Repository générique
//!
//! Chaque capacité générique est un **trait** distinct, implémenté par une **macro** dédiée.
//! Un repository n'implémente que les capacités dont il a besoin : aucune méthode
//! n'est générée « dans le vide ».
//!
//! ## Utilisation
//!
//! ```rust,ignore
//! use crate::db::repository::prelude::*;
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

use crate::common::error::ApiError;
pub use crate::common::pagination::{PaginatedResponse, PaginationParams};
use diesel::expression::BoxableExpression;
use diesel::pg::Pg;
use diesel::sql_types::Bool;
use diesel_async::AsyncPgConnection;

/// Macros disponibles pour les repositories.
pub use crate::{
    impl_base_repository, impl_count, impl_delete, impl_exists, impl_find_all, impl_find_by_id,
    impl_paginate, impl_soft_delete,
};

/// Imports nécessaires pour appeler les méthodes génériques d'un repository.
///
/// ```rust,ignore
/// use crate::db::repository::prelude::*;
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
        impl $crate::db::repository::FindAll<$model> for $repo {
            async fn find_all(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<Vec<$model>, $crate::common::error::ApiError> {
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)
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
        impl $crate::db::repository::FindById<$model, $id_type> for $repo {
            async fn find_by_id(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<Option<$model>, $crate::common::error::ApiError> {
                use diesel::{OptionalExtension, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .find(id)
                    .first::<$model>(conn)
                    .await
                    .optional()
                    .map_err($crate::common::error::ApiError::from)
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
        impl $crate::db::repository::Delete<$id_type> for $repo {
            async fn delete(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::common::error::ApiError> {
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                let rows = diesel::delete(_table::table.find(id))
                    .execute(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)?;

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
        impl $crate::db::repository::Exists<$id_type> for $repo {
            async fn exists(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::common::error::ApiError> {
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                diesel::select(diesel::dsl::exists(_table::table.find(id)))
                    .get_result::<bool>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)
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
        impl $crate::db::repository::Count for $repo {
            async fn count(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<i64, $crate::common::error::ApiError> {
                use diesel::QueryDsl;
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)
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
        impl $crate::db::repository::Paginate<$model> for $repo {
            type Table = $($table)::+::table;

            async fn find_paginated(
                conn: &mut diesel_async::AsyncPgConnection,
                params: $crate::db::repository::PaginationParams,
            ) -> Result<
                $crate::db::repository::PaginatedResponse<$model>,
                $crate::common::error::ApiError,
            > {
                use diesel::{ExpressionMethods, QueryDsl, Table};
                use diesel_async::RunQueryDsl;
                use $($table)::+ as _table;

                let total = _table::table
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)?;

                let data = _table::table
                    .order(_table::$order_col.desc())
                    .then_order_by(_table::table.primary_key().desc())
                    .limit(params.per_page)
                    .offset(params.offset())
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)?;

                Ok($crate::db::repository::PaginatedResponse::new(
                    data, total, &params,
                ))
            }

            async fn find_paginated_by<F>(
                conn: &mut diesel_async::AsyncPgConnection,
                params: $crate::db::repository::PaginationParams,
                filter: F,
            ) -> Result<
                $crate::db::repository::PaginatedResponse<$model>,
                $crate::common::error::ApiError,
            >
            where
                F: Fn() -> $crate::db::repository::Filter<Self::Table> + Send,
            {
                use diesel::{ExpressionMethods, QueryDsl, Table};
                use diesel_async::RunQueryDsl;
                use $($table)::+ as _table;

                let total = _table::table
                    .filter(filter())
                    .count()
                    .get_result::<i64>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)?;

                let data = _table::table
                    .filter(filter())
                    .order(_table::$order_col.desc())
                    .then_order_by(_table::table.primary_key().desc())
                    .limit(params.per_page)
                    .offset(params.offset())
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)?;

                Ok($crate::db::repository::PaginatedResponse::new(
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
        impl $crate::db::repository::SoftDelete<$model, $id_type> for $repo {
            async fn soft_delete(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::common::error::ApiError> {
                use diesel::{ExpressionMethods, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                let rows = diesel::update(_table::table.find(id))
                    .set(_table::deleted_at.eq(diesel::dsl::now.nullable()))
                    .execute(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)?;

                Ok(rows > 0)
            }

            async fn find_active(
                conn: &mut diesel_async::AsyncPgConnection,
            ) -> Result<Vec<$model>, $crate::common::error::ApiError> {
                use diesel::{ExpressionMethods, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                _table::table
                    .filter(_table::deleted_at.is_null())
                    .load::<$model>(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)
            }

            async fn restore(
                conn: &mut diesel_async::AsyncPgConnection,
                id: $id_type,
            ) -> Result<bool, $crate::common::error::ApiError> {
                use diesel::{ExpressionMethods, QueryDsl};
                use diesel_async::RunQueryDsl;
                use $table as _table;

                let rows = diesel::update(_table::table.find(id))
                    .set(_table::deleted_at.eq(None::<chrono::NaiveDateTime>))
                    .execute(conn)
                    .await
                    .map_err($crate::common::error::ApiError::from)?;

                Ok(rows > 0)
            }
        }
    };
}
