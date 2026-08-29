# syntax=docker/dockerfile:1.7

# 1) Frontend: build da SPA React (Vite) -> /frontend/dist
FROM node:22-alpine AS frontend
WORKDIR /frontend
COPY apps/web/package.json apps/web/package-lock.json* ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY apps/web/ ./
RUN npm run build

# 2) Backend: planner e cache de dependências Rust
FROM rust:1.88-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /build

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY apps/server/Cargo.toml ./apps/server/Cargo.toml
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS deps
COPY --from=planner /build/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

FROM chef AS backend
COPY Cargo.toml Cargo.lock ./
COPY apps/server/ ./apps/server/
# Garante que o codigo real entrou no build context.
RUN test -f apps/server/src/main.rs && grep -q "serve_application" apps/server/src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release -p ferrugem-web --features server,web-push && \
    test -x target/release/ferrugem-web

# 3) Runtime mínimo
FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates wget passwd && \
    rm -rf /var/lib/apt/lists/* && \
    useradd --system --uid 10001 --home-dir /app --shell /usr/sbin/nologin presumidos

WORKDIR /app
COPY --from=backend /build/target/release/ferrugem-web /app/ferrugem-web
COPY --from=frontend /frontend/dist /app/public
RUN mkdir -p /data /backups && chown -R presumidos:presumidos /app /data /backups

ENV STATIC_DIR=/app/public
ENV IP=0.0.0.0
ENV PORT=8080
ENV LISTEN_ADDRESS=0.0.0.0:8080
ENV PRESUMIDOS_BACKUP_DIR=/backups

EXPOSE 8080
VOLUME ["/data"]
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=10s --timeout=3s --start-period=20s --retries=6 CMD wget -qO- http://127.0.0.1:8080/health/ready >/dev/null || exit 1
USER presumidos
ENTRYPOINT ["/app/ferrugem-web"]
