//! Pagination : paramètres de query, paramètres normalisés et réponse paginée.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Query `?page=&per_page=` (validée par `ValidatedQuery`).
#[derive(Deserialize, Validate)]
pub struct PaginationQuery {
    #[validate(range(min = 1))]
    pub page: Option<i64>,
    #[validate(range(min = 1, max = 100))]
    pub per_page: Option<i64>,
}

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
// Tests
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
