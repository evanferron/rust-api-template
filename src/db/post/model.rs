use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

/// Modèle Diesel représentant une ligne de la table `posts` (lectures).
/// Non sérialisable : l'API expose le DTO `PostResponse`.
#[derive(Debug, Clone, Queryable, Selectable, Identifiable, Associations)]
#[diesel(table_name = crate::db::schema::posts)]
#[diesel(belongs_to(crate::db::user::model::User))] // Permet de faire des jointures avec User via user_id
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Modèle utilisé pour les insertions (INSERT INTO).
/// Ne contient ni `id` (`gen_random_uuid()`) ni `created_at` / `updated_at` (`NOW()`) : générés par la base.
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::db::schema::posts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewPost {
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
}

/// Modèle utilisé pour les mises à jour partielles (UPDATE).
/// Tous les champs sont optionnels — seuls les champs `Some` sont mis à jour.
#[derive(Debug, AsChangeset)]
#[diesel(table_name = crate::db::schema::posts)]
pub struct PostChangeset {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published: Option<bool>,
}
