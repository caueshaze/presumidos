FROM rust:1.88-bookworm AS builder

WORKDIR /build

RUN rustup target add wasm32-unknown-unknown && \
    cargo install dioxus-cli --version 0.7.9 --locked

COPY . .

# Garante que o codigo real entrou no build context.
RUN test -f ferrugem-web/src/main.rs && \
    grep -q "serve_application" ferrugem-web/src/main.rs

# Bundle Dioxus: builda o server e o client (wasm + assets com hash).
RUN cd ferrugem-web && dx bundle --platform web --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/dx/ferrugem-web/release/web/ /app/

ENV DIOXUS_PUBLIC_PATH=/app/public
ENV IP=0.0.0.0
ENV PORT=8080

EXPOSE 8080

ENTRYPOINT ["/app/server"]
