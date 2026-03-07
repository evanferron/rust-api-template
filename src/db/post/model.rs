use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable, Associations,
)]
#[diesel(table_name = crate::db::schema::posts)]
#[diesel(belongs_to(crate::db::user::model::User))] // Permet de faire des jointures avec User via user_id
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Modèle pour les insertions.
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::db::schema::posts)]
pub struct NewPost {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub published: bool,
}
