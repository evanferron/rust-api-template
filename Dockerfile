# =============================================================================
# Stage 1 — Chef
# =============================================================================
FROM rust:1.94-slim-bookworm AS chef

RUN apt-get update && apt-get install -y \
    libpq-dev \
    pkg-config \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

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
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --bin server

# =============================================================================
# Stage 4 — Runtime
# =============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    libpq5 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --uid 1001 --no-create-home --shell /bin/false appuser

WORKDIR /app

COPY --from=builder /app/target/release/server /app/server
COPY --from=builder /app/migrations /app/migrations

RUN chown -R appuser:appuser /app
USER appuser

ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

CMD ["/app/server"]