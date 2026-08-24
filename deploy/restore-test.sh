#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

if [ "${1:-}" = "" ]; then
  echo "Uso: deploy/restore-test.sh backups/backup-YYYYMMDDTHHMMSSZ-XXXXXXXX" >&2
  exit 1
fi

BACKUP="$1"
[ -d "$BACKUP" ] || { echo "diretorio de backup nao encontrado: $BACKUP" >&2; exit 1; }
case "$BACKUP" in
  backups/*) ;;
  *) echo "o backup precisa estar dentro de ./backups" >&2; exit 1 ;;
esac

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

echo "Construindo imagem da aplicacao..."
docker compose build ferrugem-web >/dev/null
IMAGE=presumidos/ferrugem-web:local-prod

echo "Validando e restaurando em volume isolado..."
docker run --rm --user 0 --entrypoint /app/ferrugem-web \
  -v "$SRC_DIR":/backups:ro -v "$VOL":/data \
  "$IMAGE" backup restore \
  --input "/backups/$NAME" --database /data/bolao.db --assets /data/assets

docker run --rm --user 0 \
  --env-file .env \
  -e APP_ENV=development -e DATABASE_PATH=/data/bolao.db \
  -e PRESUMIDOS_ASSET_DIR=/data/assets -e PRESUMIDOS_BACKUP_DIR=/backups \
  -e RATE_LIMIT_BACKEND=memory -e COOKIE_SECURE=false \
  -e REQUIRE_TRUSTED_PROXY=false \
  -v "$VOL":/data \
  "$IMAGE" db check

echo "Restore isolado validado. Subindo smoke HTTP em http://localhost:18080..."
docker run --rm --name "$CONTAINER" \
  --env-file .env \
  -e APP_ENV=development -e DATABASE_PATH=/data/bolao.db \
  -e PRESUMIDOS_ASSET_DIR=/data/assets -e PRESUMIDOS_BACKUP_DIR=/backups \
  -e RATE_LIMIT_BACKEND=memory -e COOKIE_SECURE=false \
  -e REQUIRE_TRUSTED_PROXY=false -e LISTEN_ADDRESS=0.0.0.0:8080 \
  -v "$VOL":/data -p 18080:8080 \
  "$IMAGE"
