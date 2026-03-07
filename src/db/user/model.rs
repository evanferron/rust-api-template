use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Modèle Diesel représentant une ligne de la table `users`.
/// Utilisé pour les lectures (SELECT).
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Modèle utilisé pour les insertions (INSERT INTO).
/// Ne contient pas created_at / updated_at — gérés par la DB (DEFAULT NOW()).
#[derive(Debug, Insertable)]
#[diesel(table_name = crate::db::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser {
    pub id: Uuid,
    pub email: String,
    pub password: String,
    pub first_name: String,
    pub last_name: String,
}

/// Modèle utilisé pour les mises à jour partielles (UPDATE).
/// Tous les champs sont optionnels — seuls les champs `Some` sont mis à jour.
#[derive(Debug, AsChangeset)]
#[diesel(table_name = crate::db::schema::users)]
pub struct UserChangeset {
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub password: Option<String>,
}
