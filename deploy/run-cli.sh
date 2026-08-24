#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

IMAGE="${PRESUMIDOS_CLI_IMAGE:-presumidos/ferrugem-web:local-prod}"
APP_CONTAINER="${PRESUMIDOS_APP_CONTAINER:-$(docker compose ps -q ferrugem-web 2>/dev/null || true)}"

if [ -z "$APP_CONTAINER" ]; then
  APP_CONTAINER="$(docker compose ps -aq ferrugem-web 2>/dev/null || true)"
fi
if [ -z "$APP_CONTAINER" ]; then
  echo "container ferrugem-web não encontrado; não é possível localizar o volume de dados" >&2
  exit 1
fi

if [ ! -f ".env" ]; then
  echo "arquivo .env não encontrado na raiz do projeto" >&2
  exit 1
fi

data_volume="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/data"}}{{.Name}}{{end}}{{end}}' "$APP_CONTAINER")"
if [ -z "$data_volume" ]; then
  data_volume="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/data"}}{{.Source}}{{end}}{{end}}' "$APP_CONTAINER")"
fi
if [ -z "$data_volume" ]; then
  echo "volume /data não encontrado no container ferrugem-web" >&2
  exit 1
fi

env_value() {
  awk -F= -v key="$1" '$1 == key {sub(/^[^=]*=/, ""); print; exit}' .env
}

public_base_url="${PUBLIC_BASE_URL:-$(env_value PUBLIC_BASE_URL)}"
if [ -z "$public_base_url" ]; then
  app_domain="${APP_DOMAIN:-$(env_value APP_DOMAIN)}"
  if [ -n "$app_domain" ]; then
    public_base_url="https://$app_domain"
  fi
fi
if [ -z "$public_base_url" ]; then
  echo "PUBLIC_BASE_URL ou APP_DOMAIN precisa estar configurado" >&2
  exit 1
fi

mkdir -p backups
chmod 700 backups

exec docker run --rm --network none --user 0 \
  --env-file .env \
  -e APP_ENV=production \
  -e DEV_DISABLE_AUTH_EMAILS=false \
  -e "PUBLIC_BASE_URL=$public_base_url" \
  -e DATABASE_PATH=/data/bolao.db \
  -e PRESUMIDOS_ASSET_DIR=/data/assets \
  -e PRESUMIDOS_BACKUP_DIR=/backups \
  -e COOKIE_SECURE=true \
  -e REQUIRE_TRUSTED_PROXY=true \
  -e TRUSTED_PROXY_CIDRS=172.31.0.10/32 \
  -e RATE_LIMIT_BACKEND=redis \
  -e REDIS_URL=redis://redis:6379 \
  -e IP=0.0.0.0 \
  -e PORT=8080 \
  -e STATIC_DIR=/app/public \
  -v "$data_volume:/data" \
  -v "$(pwd)/backups:/backups:Z" \
  --entrypoint /app/ferrugem-web \
  "$IMAGE" "$@"
