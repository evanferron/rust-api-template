# =============================================================================
# Stage 1 — Builder
# Compile le binaire en mode release avec cache des dépendances
# =============================================================================
FROM rust:1.85-slim-bookworm AS chef

WORKDIR /app

# Dépendances système pour Diesel + PostgreSQL
RUN apt-get update && apt-get install -y \
    libpq-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Astuce : copie uniquement les fichiers de dépendances en premier
# → si seul src/ change, les dépendances ne sont pas recompilées
COPY Cargo.toml Cargo.lock ./

# Crée un src/main.rs factice pour compiler les dépendances seules
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --bin server
RUN rm -rf src

# Copie le vrai code source
COPY src ./src
COPY migrations ./migrations

# Argument de version injecté par la CI
ARG APP_VERSION=dev
ENV APP_VERSION=${APP_VERSION}

# Compile le binaire final
# touch force la recompilation même si Cargo pense que rien n'a changé
RUN touch src/main.rs && cargo build --release --bin server

# =============================================================================
# Stage 2 — Runtime
# Image minimale — seulement le binaire + les libs système nécessaires
# =============================================================================
FROM debian:bookworm-slim AS runtime

WORKDIR /app

# Dépendances runtime uniquement (pas de compilateur)
RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Utilisateur non-root pour la sécurité
RUN useradd --uid 1001 --no-create-home --shell /bin/false appuser

# Copie le binaire depuis le builder
COPY --from=builder /app/target/release/server /app/server

# Migrations embarquées dans l'image
COPY --from=builder /app/migrations /app/migrations

# Permissions
RUN chown -R appuser:appuser /app
USER appuser

# Variables d'environnement par défaut (surchargées par Kubernetes)
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

# Healthcheck utilisé par Kubernetes
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

CMD ["/app/server"]