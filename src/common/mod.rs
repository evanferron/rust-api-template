//! Briques partagées utilisées par les modules : erreurs, état, extracteurs,
//! JWT, mots de passe, pagination et middlewares.
//!
//! Ne dépend ni de `app`, ni de `modules`.

pub mod error;
pub mod extract;
pub mod jwt;
pub mod middlewares;
pub mod pagination;
pub mod password;
pub mod state;
