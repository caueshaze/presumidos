FROM rust:1.88-bookworm AS builder

WORKDIR /build

COPY . .

# Garante que o código real entrou no build context.
RUN test -f ferrugem-web/src/main.rs && \
    grep -q "serve_application" ferrugem-web/src/main.rs

# Build real do servidor.
RUN cargo build --release -p ferrugem-web --features server && \
    stat -c%s target/release/ferrugem-web && \
    test "$(stat -c%s target/release/ferrugem-web)" -gt 1000000

FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/ferrugem-web /app/ferrugem-web
COPY --from=builder /build/ferrugem-web/assets /app/public/assets

ENV DIOXUS_PUBLIC_PATH=/app/public
ENV IP=0.0.0.0
ENV PORT=8080

EXPOSE 8080

CMD ["/app/ferrugem-web"]