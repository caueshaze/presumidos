#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

if [ "${1:-}" = "" ]; then
  echo "Uso: deploy/restore-test.sh backups/ferrugem-YYYYMMDD-HHMMSS.db" >&2
  exit 1
fi

BACKUP="$1"
[ -f "$BACKUP" ] || { echo "arquivo nao encontrado: $BACKUP" >&2; exit 1; }

VOL=ferrugem_restore_test_data
CONTAINER=ferrugem_restore_test
NAME=$(basename "$BACKUP")
SRC_DIR=$(cd "$(dirname "$BACKUP")" && pwd)

cleanup() {
  echo "Limpando ambiente de teste..."
  docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
  docker volume rm -f "$VOL" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

cleanup

docker volume create "$VOL" >/dev/null

echo "Copiando $BACKUP para volume de teste..."
docker run --rm -v "$SRC_DIR":/src:ro -v "$VOL":/data alpine \
  cp "/src/$NAME" /data/bolao.db

echo "Construindo imagem da aplicacao..."
docker compose build ferrugem-web >/dev/null
IMAGE=$(docker compose images -q ferrugem-web)

echo "Subindo app de teste em http://localhost:18080 (Ctrl+C para encerrar)..."
echo "Ambiente isolado: volume e container temporarios, nao usa app_data nem a rede origin."

docker run --rm --name "$CONTAINER" \
  --env-file .env \
  -e APP_ENV=development \
  -e DATABASE_PATH=/data/bolao.db \
  -e RATE_LIMIT_BACKEND=memory \
  -e COOKIE_SECURE=false \
  -e REQUIRE_TRUSTED_PROXY=false \
  -e IP=0.0.0.0 \
  -e PORT=8080 \
  -e DIOXUS_PUBLIC_PATH=/app/public \
  -v "$VOL":/data \
  -p 18080:8080 \
  "$IMAGE"
