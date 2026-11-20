# rust-api-template

Template API REST production-ready — Axum 0.8 · Diesel 2 async · PostgreSQL · JWT

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=alert_status&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=coverage&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)

---

## Prérequis

- Rust 1.89+
- Docker (pour PostgreSQL)
- `diesel_cli` : `cargo install diesel_cli --no-default-features --features postgres`

---

## Démarrage rapide

```bash
# 1. Cloner le projet
git clone https://github.com/<ton-org>/rust-api-template
cd rust-api-template

# 2. Lancer PostgreSQL
docker-compose up -d

# 3. Configurer l'environnement
cp .env.example .env
# Éditer .env avec tes valeurs

# 4. Lancer les migrations
diesel migration run

# 5. Démarrer le serveur
cargo run

# API disponible sur http://localhost:8080
# Swagger UI sur http://localhost:8080/swagger-ui
```

---

## Stack

| Composant | Technologie |
|-----------|-------------|
| Framework | Axum 0.8 |
| ORM | Diesel 2.4 + diesel-async 0.10 |
| Base de données | PostgreSQL 16 |
| Auth | JWT HS256 + refresh token (cookie HttpOnly) |
| Validation | validator 0.21 |
| Doc API | utoipa + Swagger UI |
| Rate limiting | governor (par IP + par user) |
| Logs | tracing (pretty dev / JSON prod) |

---

## Architecture

```
src/
├── core/          ← erreurs, validation, middlewares, repository générique
├── db/            ← modèles et repositories Diesel (un dossier par entité)
├── infra/         ← Config, AppState
├── launch/        ← assemblage du router, Swagger
└── modules/       ← logique métier (auth, user, post...)
```

---

## Ajouter un module

Prendre le module `post` comme référence (`src/db/post/` + `src/modules/post/`) :

1. Créer la migration : `diesel migration generate create_<nom>s`
2. `diesel migration run` (régénère `src/db/schema.rs`)
3. Créer `src/db/<nom>/` (model + repository) et `src/modules/<nom>/` (dto, service, handler, routes)
4. Ajouter `pub mod <nom>;` dans `src/db/mod.rs` et `src/modules/mod.rs`
5. Brancher les routes dans `src/launch/router.rs` et les déclarer dans `src/launch/swagger.rs`

---

## Tests

```bash
docker-compose up -d postgres_test   # base de test sur le port 5433

cargo test                           # unitaires + intégration + Cucumber
cargo llvm-cov --html                # idem avec rapport de couverture (ou .\script\cov.ps1)
```

> Les tests utilisent une base dédiée (`DATABASE_TEST_URL`) et appliquent eux-mêmes les migrations.
> `tests/repository.rs` tourne dans des transactions annulées ; Cucumber réinitialise la base avant chaque scénario.

---

## Variables d'environnement

Voir `.env.example` pour la liste complète. Les variables essentielles :

```env
DATABASE_URL=postgres://postgres:password@localhost:5432/app_db
DATABASE_TEST_URL=postgres://postgres:password@localhost:5433/app_test_db
JWT_SECRET=<générer avec openssl rand -base64 64>
JWT_REFRESH_SECRET=<générer avec openssl rand -base64 64>
```

---

## CI

`.github/workflows/ci.yml` — sur chaque push et pull request, trois jobs en parallèle :

| Job | Contrôle |
|-----|----------|
| Format & Clippy | `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` |
| Tests & Coverage | tous les tests sur PostgreSQL 16 + couverture (`cargo llvm-cov`) → SonarCloud |
| Security audit | `cargo audit` (base RustSec) |

Pour reproduire la CI en local :

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo audit
```

Le scan SonarCloud ne s'exécute que si le secret `SONAR_TOKEN` est configuré.

---

## Documentation Rust

```bash
cargo doc --no-deps --document-private-items --open
```

Le fonctionnement du repository générique est documenté dans le module `core::repository`.

