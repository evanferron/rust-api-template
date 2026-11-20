//! # Organisation
//!
//! | Dossier | Rôle |
//! |---------|------|
//! | `config.rs` | Configuration (variables d'environnement) |
//! | `app/` | Assemblage et démarrage : router, layers, OpenAPI, logs, serveur |
//! | `common/` | Briques partagées : erreurs, état, extracteurs, JWT, pagination, middlewares |
//! | `db/` | Données : pool, migrations, schéma, repository générique, model + repository par table |
//! | `modules/` | Fonctionnalités HTTP : un dossier par module (mod.rs = routes, handler, service, dto) |
//!
//! Sens des dépendances : `app → modules → (common, db)`, `db → common`, `common → config`.
//! Rien n'importe `app`, à part `main.rs` et les tests.

pub mod app;
pub mod common;
pub mod config;
pub mod db;
pub mod modules;
