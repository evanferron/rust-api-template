# =============================================================================
# Stage 1 — Chef
# =============================================================================
# Version de Rust : >= rust-version du Cargo.toml (MSRV)
FROM rust:1.100-slim-bookworm AS chef

RUN cargo install cargo-chef --locked

WORKDIR /app

# =============================================================================
# Stage 2 — Planner
# =============================================================================
FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Stage 3 — Builder
# =============================================================================
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json

COPY . .
RUN cargo build --release --locked --bin server

# =============================================================================
# Stage 4 — Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 1001 --no-create-home --shell /bin/false appuser

WORKDIR /app

# Les migrations sont embarquées dans le binaire (embed_migrations!)
COPY --from=builder /app/target/release/server /app/server

RUN chown -R appuser:appuser /app
USER appuser

ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV ENVIRONMENT=production
ENV RUST_LOG=info

EXPOSE 8080

# Liveness uniquement : la readiness (/api/health/ready) est pour l'orchestrateur / le load balancer
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/health/live || exit 1

# SIGTERM déclenche l'arrêt propre (requêtes en cours terminées, SHUTDOWN_TIMEOUT_SECS max)
STOPSIGNAL SIGTERM
CMD ["/app/server"]