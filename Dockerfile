FROM rust:1.88-bookworm AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY ferrugem-web/Cargo.toml ferrugem-web/Cargo.toml
RUN mkdir -p ferrugem-web/src && \
    printf "fn main() {}\n" > ferrugem-web/src/main.rs && \
    cargo build --release -p ferrugem-web --features server && \
    rm -rf ferrugem-web/src

COPY . .

# Garante que o codigo real entrou no build context.
RUN test -f ferrugem-web/src/main.rs && \
    grep -q "serve_application" ferrugem-web/src/main.rs

# Remove o binario dummy antes do build real.
RUN cargo clean -p ferrugem-web && \
    cargo build --release -p ferrugem-web --features server && \
    test "$(stat -c%s target/release/ferrugem-web)" -gt 1000000

FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/ferrugem-web /app/ferrugem-web
COPY ferrugem-web/assets /app/public/assets

ENV DIOXUS_PUBLIC_PATH=/app/public
ENV IP=0.0.0.0
ENV PORT=8080

EXPOSE 8080

CMD ["/app/ferrugem-web"]
