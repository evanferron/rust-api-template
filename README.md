# rust-api-template

Template API REST production-ready — Axum 0.8 · Diesel 2 async · PostgreSQL · JWT

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=alert_status&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=coverage&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)

---

## Prérequis

- Rust 1.85+
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
| ORM | Diesel 2 + diesel-async 0.7 |
| Base de données | PostgreSQL 16 |
| Auth | JWT HS256 + refresh token (cookie HttpOnly) |
| Validation | validator 0.19 |
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
├── modules/       ← logique métier (auth, user, post...)
└── bin/
    └── generate.rs ← générateur de modules
```

---

## Ajouter un module

```bash
cargo run --bin generate -- generate <nom>
```

Génère automatiquement la migration, le modèle, le repository, les DTOs, le service, le handler et les routes.

Ensuite :

1. Compléter `migrations/.../up.sql`
2. `diesel migration run`
3. Ajouter `pub mod <nom>;` dans `src/db/mod.rs` et `src/modules/mod.rs`
4. Brancher dans `src/launch/router.rs`

---

## Tests

```bash
# Tests unitaires + e2e
cargo test -- --test-threads=1

# Coverage HTML
cargo llvm-cov --all --html --open -- --test-threads=1
```

> Les tests e2e utilisent une base dédiée (`DATABASE_TEST_URL`) et se réinitialisent avant chaque test.

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

## CI/CD

| Pipeline | Déclencheur | Étapes |
|----------|-------------|--------|
| `build.yml` | Push / PR sur `main`, `develop` | Lint → Tests → Coverage → SonarCloud → cargo audit |
| `cd.yml` | Push sur `main` | Docker Hub → Render deploy |

**Secrets GitHub requis :**
`DOCKERHUB_USERNAME` · `DOCKERHUB_TOKEN` · `RENDER_DEPLOY_HOOK_URL` · `SONAR_TOKEN`

---

## Assistance IA

Ce projet inclut des fichiers de contexte pour maximiser l'aide des outils IA :

| Fichier | Outil | Activation |
|---------|-------|------------|
| `.github/copilot-instructions.md` | GitHub Copilot | Automatique |
| `docs/IA_CONTEXT.md` | Claude / ChatGPT | Coller en début de conversation |

`docs/IA_CONTEXT.md` contient l'architecture complète, tous les patterns de code, les décisions techniques et les pièges connus — colle-le dans n'importe quel chat IA pour obtenir une assistance adaptée au projet.
