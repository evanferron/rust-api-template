# rust-api-template

Template d'API REST Rust prête pour la production — Axum 0.8 · Diesel 2.4 async · PostgreSQL 18 · JWT

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=alert_status&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=evanferron_rust-api-template&metric=coverage&branch=main)](https://sonarcloud.io/summary/new_code?id=evanferron_rust-api-template&branch=main)

> Ce README s'adresse aux développeurs. Les conventions détaillées à l'usage des agents IA
> (recettes pas à pas, pièges, règles strictes) sont dans [`AGENTS.md`](AGENTS.md).

---

## Prérequis

- Rust 1.89+ (`rust-version` du `Cargo.toml`)
- Docker (PostgreSQL en local)
- `diesel_cli`, pour créer des migrations et régénérer `src/db/schema.rs` :
  `cargo install diesel_cli --no-default-features --features postgres-bundled`

L'API n'a pas besoin de `libpq` : tout passe par `diesel-async` (driver 100 % Rust), migrations comprises.
Seul `diesel_cli` utilise libpq, compilée et embarquée par la feature `postgres-bundled`.

---

## Démarrage rapide

```bash
# 1. Créer un dépôt depuis ce template (bouton « Use this template » sur GitHub), puis le cloner

# 2. Lancer PostgreSQL (dev sur 5432, tests sur 5433)
docker compose up -d

# 3. Configurer l'environnement
cp .env.example .env

# 4. Démarrer le serveur (les migrations sont appliquées au démarrage)
cargo run
```

- API : http://localhost:8080/api
- Swagger UI : http://localhost:8080/swagger-ui (spécification : `/api-docs/openapi.json`)

Tout en conteneur, API comprise : `docker compose --profile app up --build`.

---

## Endpoints fournis

Tous sous `/api`. La documentation complète (paramètres, corps, réponses) est dans le Swagger UI.

| Module | Routes | Authentification |
|--------|--------|------------------|
| health | `GET /health/live` · `GET /health/ready` | non |
| auth | `POST /auth/register` · `POST /auth/login` · `POST /auth/refresh` · `POST /auth/logout` | non (limite stricte par IP sur register / login / refresh) |
| users | `GET /users` (paginé) · `GET /users/{id}` · `PUT /users/{id}` · `DELETE /users/{id}` | Bearer JWT ; modification et suppression de son propre compte uniquement |
| posts | `GET /posts` (paginé, ses propres posts) · `POST /posts` · `GET /posts/{id}` · `PUT /posts/{id}` · `DELETE /posts/{id}` | Bearer JWT ; modification et suppression de ses propres posts uniquement |

`users` et `posts` sont des modules d'exemple : `posts` sert de modèle pour les nouveaux modules.

**Authentification** : `login` renvoie un access token (en-tête `Authorization: Bearer <token>`) et pose le
refresh token dans un cookie HttpOnly limité à `/api/auth/refresh`.

**Pagination** : `?page=1&per_page=20` (max 100) → `{ data, total, page, per_page, total_pages }`.

**Erreurs** : format RFC 9457 (`application/problem+json`) avec un `code` stable, par exemple :

```json
{ "type": "about:blank", "title": "Not Found", "status": 404,
  "code": "POST_NOT_FOUND", "detail": "Post not found", "request_id": "4f1c…" }
```

Les erreurs de validation (422 `VALIDATION_FAILED`) détaillent les champs dans `errors`.
Le catalogue des codes est dans `src/common/error.rs` (macro `error_codes!`) et dans le schéma `ErrorCode` du Swagger.

---

## Configuration

Toutes les variables sont documentées dans [`.env.example`](.env.example) avec leur valeur par défaut.
Seules trois sont obligatoires :

```env
DATABASE_URL=postgres://postgres:password@localhost:5432/app_db
JWT_SECRET=<générer avec openssl rand -base64 64>
JWT_REFRESH_SECRET=<générer avec openssl rand -base64 64>
```

Les tests utilisent en plus `DATABASE_TEST_URL` (base sur le port 5433).

Au démarrage, la configuration est validée en une fois : variable obligatoire absente, valeur invalide
(`SERVER_PORT=abc`) ou ancien nom de variable (`JWT_EXPIRATION` → `JWT_EXPIRATION_SECS`) arrêtent le serveur
avec la liste de toutes les erreurs.

---

## Exploitation

| Sujet | Comportement |
|-------|--------------|
| Sondes | `GET /api/health/live` : le process répond · `GET /api/health/ready` : base joignable (503 si injoignable ou pendant l'arrêt) |
| Arrêt | `SIGTERM` / Ctrl+C : plus de nouvelles connexions, requêtes en cours terminées (au plus `SHUTDOWN_TIMEOUT_SECS`) |
| Logs | `ENVIRONMENT=production` → JSON, une ligne par requête · `development` → texte lisible · niveau via `RUST_LOG` |
| Traçabilité | `x-request-id` repris s'il est fourni, sinon généré ; présent dans les logs, l'en-tête de réponse et les erreurs |
| Rate limiting | `RATE_LIMIT_RPM` par utilisateur (ou IP si anonyme) sur toute l'API · `RATE_LIMIT_AUTH_RPM` par IP sur register / login / refresh · 429 avec `Retry-After` |
| Reverse proxy | `TRUST_PROXY=true` pour lire l'IP client dans `X-Forwarded-For` (uniquement si l'API n'est joignable que via le proxy) |
| Docker | image multi-stage, utilisateur non root, `HEALTHCHECK` sur `/api/health/live`, migrations embarquées dans le binaire |

---

## Stack

| Composant | Technologie |
|-----------|-------------|
| Framework | Axum 0.8 |
| ORM | Diesel 2.4 + diesel-async 0.10 (pool bb8) |
| Base de données | PostgreSQL 18 |
| Auth | JWT HS256 + refresh token (cookie HttpOnly), bcrypt |
| Validation | validator 0.21 |
| Doc API | utoipa 6 + utoipa-axum + Swagger UI |
| Rate limiting | governor (par utilisateur ou IP + limite stricte sur l'authentification) |
| Logs | tracing (texte en dev, JSON en prod) |

---

## Architecture

```
src/
├── main.rs        point d'entrée : charge la config, lance le serveur
├── lib.rs
├── config.rs      configuration (variables d'environnement)
├── app/           assemblage et démarrage : router, couches HTTP, OpenAPI, logs, serveur
├── common/        partagé : erreurs, état, extracteurs, JWT, mots de passe, pagination, middlewares
├── db/            pool, migrations, schéma, repository générique, model + repository par table
└── modules/       fonctionnalités HTTP (auth, user, post…) : mod.rs (routes), handler, service, dto
```

Sens des dépendances : `app → modules → (common, db)`, `db → common`, `common → config`.
Rien n'importe `app` (sauf `main.rs` et les tests), et **aucun module n'importe un autre module**.

```mermaid
flowchart TD
    %% ───────────── Points d'entrée ─────────────
    main["<b>main.rs</b><br/>charge .env + Config<br/>lance app::server::run"]
    tests["<b>tests/</b><br/>Cucumber · repository · health<br/>utilisent build_app et db"]

    %% ───────────── Racine ─────────────
    config["<b>config.rs</b><br/>Config : Server · Http · RateLimit · Database · Jwt<br/>ConfigError : toutes les erreurs d'un coup"]

    %% ───────────── app/ ─────────────
    subgraph APP["app/ — assemblage et démarrage (importé seulement par main et tests)"]
        direction TB
        server["<b>server.rs</b><br/>logs → migrations → pool<br/>→ AppState → écoute HTTP<br/>arrêt propre (SIGTERM / Ctrl+C)"]
        appmod["<b>mod.rs</b><br/>api_router : merge des modules sous /api<br/>build_app : couches, fallbacks, Swagger"]
        openapi["<b>openapi.rs</b><br/>infos générales, tags, schéma Bearer"]
        telemetry["<b>telemetry.rs</b><br/>logs texte en dev, JSON en prod"]
        server --> appmod
        server --> telemetry
        appmod --> openapi
    end

    %% ───────────── modules/ ─────────────
    subgraph MODULES["modules/ — fonctionnalités HTTP (même format, aucun import entre modules)"]
        direction LR
        health["<b>health</b>"]
        auth["<b>auth</b>"]
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
        password["<b>password.rs</b><br/>hash_password · verify_password (bcrypt)"]
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
        password --> error
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
    appmod --> COMMON
    server --> dbmod
    server --> state

    MODULES --> COMMON
    MODULES --> entities

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
    class error,extract,jwt,password,pagination,state,mw common
    class dbmod,repo,schema,entities db

    style APP     fill:#eff6ff,stroke:#1d4ed8,stroke-width:2px,color:#1e3a8a
    style MODULES fill:#f0fdf4,stroke:#15803d,stroke-width:2px,color:#14532d
    style COMMON  fill:#fdf2f8,stroke:#be185d,stroke-width:2px,color:#831843
    style DB      fill:#fff7ed,stroke:#c2410c,stroke-width:2px,color:#7c2d12

    linkStyle default stroke:#64748b,stroke-width:1.5px
```

- Les flèches vont toujours vers le bas : rien ne pointe vers `app` ni vers un module.
- `app/` câble l'application, sans logique métier.
- `common/` est le vocabulaire partagé des modules (erreurs, authentification, validation, pagination).
- `db/` ignore tout du HTTP.

---

## Développer

### Ajouter un module

Le module `post` (`src/modules/post/` + `src/db/post/`) sert de modèle.

1. Si le module a une table : `diesel migration generate create_<table>s`, écrire `up.sql` / `down.sql`, puis
   `diesel migration run` (régénère `src/db/schema.rs`) ; créer `src/db/<table>/` (`model.rs`, `repository.rs`)
   et le déclarer dans `src/db/mod.rs`.
2. Créer `src/modules/<module>/` (`mod.rs`, `handler.rs`, `service.rs`, `dto.rs`) et le déclarer dans `src/modules/mod.rs`.
3. Ajouter `.merge(<module>::routes())` dans `api_router()` (`src/app/mod.rs`).
4. Si le module introduit un nouveau tag OpenAPI, ajouter sa description dans `src/app/openapi.rs`.

Le chemin, la méthode et la documentation OpenAPI d'une route viennent tous de `#[utoipa::path]` sur le handler,
enregistré par `routes!(...)` dans le `mod.rs` du module. Les conventions de code (rôle de chaque fichier,
erreurs, validation, pagination, migrations) sont détaillées dans [`AGENTS.md`](AGENTS.md).

### Ajouter un code d'erreur

Une ligne dans `error_codes!` (`src/common/error.rs`) : `CommentNotFound => (NOT_FOUND, "Comment not found"),`.
Le nom JSON (`COMMENT_NOT_FOUND`), le statut et le schéma OpenAPI en découlent.

### Migrations

Une migration déjà appliquée sur un environnement partagé ne se modifie pas : on en crée une nouvelle.
Après modification d'une migration en local, vider la base locale (`docker compose down -v && docker compose up -d`).

---

## Tests

```bash
docker compose up -d postgres_test   # base de test sur le port 5433 (DATABASE_TEST_URL)

cargo test                           # unitaires + intégration + Cucumber
cargo llvm-cov --html                # idem avec rapport de couverture (ou .\script\cov.ps1)
```

| Type | Emplacement |
|------|-------------|
| Unitaires | `#[cfg(test)] mod tests` dans chaque fichier |
| Repository générique | `tests/repository.rs` (transactions annulées : aucune donnée persistée) |
| Sondes de santé | `tests/health.rs` |
| E2E | `tests/features/*.feature` (Gherkin, en français) + `tests/steps/` (Cucumber), base réinitialisée à chaque scénario |

Les tests appliquent eux-mêmes les migrations et utilisent le même router que la production (`build_app`).

---

## CI

| Workflow | Déclencheur | Contrôle | Bloquant |
|----------|-------------|----------|----------|
| `ci.yml` — Format & Clippy | chaque push / PR | `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` | oui |
| `ci.yml` — Tests & Coverage | chaque push / PR | tous les tests sur PostgreSQL 18 + couverture (`cargo llvm-cov`) → SonarCloud | oui |
| `ci.yml` — MSRV | chaque push / PR | `cargo check` avec la version minimale de Rust (`rust-version` du `Cargo.toml`) | oui |
| `ci.yml` — Docker build | chaque push / PR | construction de l'image (sans push) | oui |
| `security-audit.yml` | quotidien · modification de `Cargo.toml` / `Cargo.lock` · manuel | `cargo audit` (base RustSec) | non |

Un seul run par branche : le push et la pull request d'une même branche partagent un groupe de concurrence.

L'audit est séparé de la CI pour qu'une vulnérabilité découverte dans une dépendance n'empêche pas de livrer
un hotfix : le workflow passe au rouge (notification GitHub) et la correction arrive par une PR Dependabot.
Ne pas le déclarer comme *required status check*.

Reproduire la CI en local :

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test && cargo audit
```

Le scan SonarCloud ne s'exécute que si le secret `SONAR_TOKEN` est configuré. Pour un projet créé depuis le
template : adapter `sonar.projectKey` et `sonar.organization` dans `sonar-project.properties`, ainsi que les
badges en haut de ce README.

### Dependabot

`.github/dependabot.yml` ouvre chaque lundi des PR de mise à jour :

- **crates** : mineures et correctifs regroupés dans une PR, montées majeures séparées ;
- **actions GitHub** : regroupées dans une PR ;
- **images Docker** (`rust`, `debian`) du `Dockerfile`.

À activer une fois dans *Settings → Code security* : **Dependabot alerts** et **Dependabot security updates**
(PR immédiates pour les dépendances vulnérables).

---

## Assistance IA

[`AGENTS.md`](AGENTS.md) décrit, pour les agents IA, l'architecture, les règles de dépendance, les recettes
(module, table, code d'erreur, tests…) et les pièges du projet. Il est lu automatiquement par la plupart des
agents (Codex, Cursor, Zed…) ; `CLAUDE.md` l'importe pour Claude Code. À mettre à jour quand une convention change.

---

## Documentation Rust

```bash
cargo doc --no-deps --document-private-items --open
```

Le fonctionnement du repository générique est documenté dans le module `db::repository`.
