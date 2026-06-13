# syntax=docker/dockerfile:1

# 1) Frontend: build da SPA React (Vite) -> /frontend/dist
FROM node:22-alpine AS frontend
WORKDIR /frontend
COPY web/package.json web/package-lock.json* ./
RUN npm ci
COPY web/ ./
RUN npm run build

# 2) Backend: build do servidor Axum (sem dx, sem WASM)
FROM rust:1.88-bookworm AS backend
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY ferrugem-web/ ./ferrugem-web/
# Garante que o codigo real entrou no build context.
RUN test -f ferrugem-web/src/main.rs && grep -q "serve_application" ferrugem-web/src/main.rs
RUN cargo build --release -p ferrugem-web --features server && \
    test -x target/release/ferrugem-web

# 3) Runtime mínimo
FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=backend /build/target/release/ferrugem-web /app/ferrugem-web
COPY --from=frontend /frontend/dist /app/public

ENV STATIC_DIR=/app/public
ENV IP=0.0.0.0
ENV PORT=8080

EXPOSE 8080
ENTRYPOINT ["/app/ferrugem-web"]
