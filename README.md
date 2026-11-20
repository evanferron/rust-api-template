# rust-api-template

Template API REST production-ready — Axum 0.8 · Diesel 2 async · PostgreSQL · JWT

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=alert_status&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=coverage&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)

---

## Prérequis

- Rust 1.89+
- Docker (pour PostgreSQL)
- `diesel_cli` (création de migrations, régénération de `schema.rs`) :
  `cargo install diesel_cli --no-default-features --features postgres-bundled`

> L'API n'a pas besoin de `libpq` : tout passe par `diesel-async` (driver 100 % Rust).
> Seul `diesel_cli` utilise libpq, compilée et embarquée grâce à la feature `postgres-bundled`.

---

## Démarrage rapide

```bash
# 1. Créer un dépôt depuis ce template (bouton « Use this template » sur GitHub), puis le cloner

# 2. Lancer PostgreSQL (dev sur 5432, tests sur 5433)
docker compose up -d

# 3. Configurer l'environnement (toutes les variables sont documentées dans .env.example)
cp .env.example .env

# 4. Démarrer le serveur (les migrations sont appliquées au démarrage)
cargo run

# API disponible sur http://localhost:8080
# Swagger UI sur http://localhost:8080/swagger-ui
```

Tout en conteneur (API comprise) : `docker compose --profile app up --build`.

### Exploitation

| Sujet | Comportement |
|-------|--------------|
| Sondes | `GET /api/health/live` (process) · `GET /api/health/ready` (base joignable, 503 pendant l'arrêt) |
| Arrêt | `SIGTERM` / Ctrl+C : plus de nouvelles connexions, requêtes en cours terminées (max `SHUTDOWN_TIMEOUT_SECS`) |
| Logs | `ENVIRONMENT=production` → JSON, une ligne par requête avec `request_id` · niveau via `RUST_LOG` |
| Erreurs | format RFC 9457 (`application/problem+json`) avec un `code` stable · catalogue dans `src/common/error.rs` |
| Reverse proxy | `TRUST_PROXY=true` pour lire l'IP client dans `X-Forwarded-For` (rate limit) |

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
├── main.rs
├── lib.rs
├── config.rs      ← configuration (variables d'environnement)
├── app/           ← assemblage et démarrage : router, layers, OpenAPI, logs, serveur
├── common/        ← partagé : erreurs, état, extracteurs, JWT, pagination, middlewares
├── db/            ← pool, migrations, schéma, repository générique, model + repository par table
└── modules/       ← fonctionnalités HTTP (auth, user, post…) : mod.rs (routes), handler, service, dto
```

Sens des dépendances : `app → modules → (common, db)`, `db → common`. Rien n'importe `app`.

```mermaid
flowchart TD
    %% ───────────── Points d'entrée ─────────────
    main["<b>main.rs</b><br/>charge .env + Config<br/>lance app::server::run"]
    tests["<b>tests/</b><br/>Cucumber + repository<br/>utilisent build_app et db"]

    %% ───────────── Racine ─────────────
    config["<b>config.rs</b><br/>Config · ServerConfig · DatabaseConfig · JwtConfig<br/>ConfigError : toutes les variables invalides d'un coup"]

    %% ───────────── app/ ─────────────
    subgraph APP["app/ — assemblage et démarrage (importé seulement par main et tests)"]
        direction TB
        server["<b>server.rs</b><br/>logs → migrations → pool<br/>→ AppState → écoute HTTP<br/>arrêt propre (SIGTERM / Ctrl+C)"]
        appmod["<b>mod.rs</b><br/>api_router : merge des modules sous /api<br/>build_app : layers, fallbacks, Swagger"]
        openapi["<b>openapi.rs</b><br/>infos générales, tags, schéma Bearer"]
        telemetry["<b>telemetry.rs</b><br/>logs pretty en dev, JSON en prod"]
        server --> appmod
        server --> telemetry
        appmod --> openapi
    end

    %% ───────────── modules/ ─────────────
    subgraph MODULES["modules/ — fonctionnalités HTTP (même format pour tous)"]
        direction LR
        health["<b>health</b>"]
        auth["<b>auth</b><br/>+ helpers.rs (bcrypt)"]
        user["<b>user</b>"]
        post["<b>post</b>"]
        fmt["chaque module :<br/>mod.rs = routes() via utoipa-axum<br/>handler → service → dto"]
    end

    %% ───────────── common/ ─────────────
    subgraph COMMON["common/ — briques partagées (n'importe ni app ni modules)"]
        direction TB
        error["<b>error.rs</b><br/>ApiError · ErrorCode (error_codes!)<br/>réponses RFC 9457"]
        extract["<b>extract.rs</b><br/>AuthUser · ValidatedJson/Query/Path · UuidParam"]
        jwt["<b>jwt.rs</b><br/>Claims · create/verify token · bearer_token"]
        pagination["<b>pagination.rs</b><br/>PaginationQuery · PaginationParams · PaginatedResponse"]
        state["<b>state.rs</b><br/>AppState : pool, config,<br/>rate limiters, shutting_down"]
        mw["<b>middlewares/</b><br/>rate_limit (global + AuthRateLimit) · request_id"]
        extract --> jwt
        extract --> state
        mw --> jwt
        mw --> state
        extract --> error
        mw --> error
        jwt --> error
    end

    %% ───────────── db/ ─────────────
    subgraph DB["db/ — données (indépendant des modules : une table peut n'avoir aucun module)"]
        direction TB
        dbmod["<b>mod.rs</b><br/>pool bb8 · migrations embarquées<br/>diesel-async, sans libpq"]
        repo["<b>repository.rs</b><br/>traits FindAll, FindById, Delete, Paginate…<br/>macros impl_*!"]
        schema["<b>schema.rs</b><br/>généré par diesel_cli"]
        entities["<b>user/ · post/</b><br/>model.rs + repository.rs"]
        entities --> repo
        entities --> schema
    end

    %% ───────────── Dépendances entre couches ─────────────
    main --> config
    main --> server
    tests --> appmod
    tests --> dbmod

    appmod --> MODULES
    server --> dbmod
    server --> state

    MODULES --> COMMON
    MODULES --> entities
    user -. "hash/verify_password" .-> auth

    repo --> error
    repo --> pagination
    dbmod --> config
    state --> config

    %% ───────────── Style ─────────────
    %% Couleurs explicites partout : fonds clairs + texte foncé, traits gris moyen,
    %% lisible avec le thème clair comme avec le thème sombre de GitHub.
    classDef entry   fill:#fde68a,stroke:#b45309,stroke-width:2px,color:#1c1917
    classDef root    fill:#c7d2fe,stroke:#4338ca,stroke-width:2px,color:#1e1b4b
    classDef app     fill:#bfdbfe,stroke:#1d4ed8,color:#0f172a
    classDef modules fill:#bbf7d0,stroke:#15803d,color:#052e16
    classDef note    fill:#f0fdf4,stroke:#15803d,stroke-dasharray:4 3,color:#052e16
    classDef common  fill:#fbcfe8,stroke:#be185d,color:#500724
    classDef db      fill:#fed7aa,stroke:#c2410c,color:#431407

    class main,tests entry
    class config root
    class server,appmod,openapi,telemetry app
    class health,auth,user,post modules
    class fmt note
    class error,extract,jwt,pagination,state,mw common
    class dbmod,repo,schema,entities db

    style APP     fill:#eff6ff,stroke:#1d4ed8,stroke-width:2px,color:#1e3a8a
    style MODULES fill:#f0fdf4,stroke:#15803d,stroke-width:2px,color:#14532d
    style COMMON  fill:#fdf2f8,stroke:#be185d,stroke-width:2px,color:#831843
    style DB      fill:#fff7ed,stroke:#c2410c,stroke-width:2px,color:#7c2d12

    linkStyle default stroke:#64748b,stroke-width:1.5px
```

- **Flèches pleines** : elles vont toujours vers le bas, jamais vers `app` ni `modules`.
- **`app/`** câble l'application, sans logique métier.
- **`common/`** est le vocabulaire partagé des modules (erreurs, auth, validation, pagination).
- **`db/`** ignore tout du HTTP.
- **Flèche pointillée** : seule dépendance entre modules (`user` → `auth::helpers` pour le hachage des mots de passe).

---

## Ajouter un module

Prendre le module `post` comme référence (`src/db/post/` + `src/modules/post/`) :

1. Créer la migration : `diesel migration generate create_<nom>s`
2. `diesel migration run` (régénère `src/db/schema.rs`)
3. Créer `src/db/<nom>/` (model + repository) et `src/modules/<nom>/` (dto, service, handler, routes)
4. Ajouter `pub mod <nom>;` dans `src/db/mod.rs` et `src/modules/mod.rs`
5. Ajouter `.merge(<nom>::routes())` dans `api_router()` (`src/app/mod.rs`)

Les routes sont déclarées avec `#[utoipa::path]` sur chaque handler et enregistrées via `routes!(...)` dans le `mod.rs` du module : le chemin, la méthode et la documentation OpenAPI viennent de la même déclaration.

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

| Workflow | Déclencheur | Contrôle | Bloquant |
|----------|-------------|----------|----------|
| `ci.yml` — Format & Clippy | chaque push / PR | `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` | oui |
| `ci.yml` — Tests & Coverage | chaque push / PR | tous les tests sur PostgreSQL 16 + couverture (`cargo llvm-cov`) → SonarCloud | oui |
| `security-audit.yml` | quotidien · modification de `Cargo.toml` / `Cargo.lock` · manuel | `cargo audit` (base RustSec) | non |

L'audit est séparé de la CI pour qu'une vulnérabilité découverte dans une dépendance n'empêche pas de livrer un hotfix : le workflow passe au rouge (notification GitHub) et la correction arrive par une PR Dependabot. Ne pas le déclarer comme *required status check*.

Pour reproduire la CI en local :

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo audit
```

Le scan SonarCloud ne s'exécute que si le secret `SONAR_TOKEN` est configuré.

### Dependabot

`.github/dependabot.yml` ouvre chaque lundi des PR de mise à jour :

- **crates** : mineures et correctifs regroupés dans une PR, montées majeures séparées ;
- **actions GitHub** : regroupées dans une PR ;
- **images Docker** (`rust`, `debian`) du `Dockerfile`.

À activer une fois dans *Settings → Code security* : **Dependabot alerts** et **Dependabot security updates** (PR immédiates pour les dépendances vulnérables).

-----|----------|
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

Le fonctionnement du repository générique est documenté dans le module `db::repository`.

