#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

IMAGE_NAME="presumidos/ferrugem-web:local-prod"
ROLLBACK_IMAGE="presumidos/ferrugem-web:rollback"
HEALTH_URL="http://ferrugem-web:8080/api/health"
HEALTH_TRIES="${HEALTH_TRIES:-20}"
HEALTH_SLEEP_SECONDS="${HEALTH_SLEEP_SECONDS:-2}"

if [ ! -f ".env" ]; then
  echo "arquivo .env nao encontrado na raiz do projeto" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker nao encontrado no PATH" >&2
  exit 1
fi

CURRENT_CONTAINER_ID="$(docker compose ps -q ferrugem-web || true)"
if [ -z "$CURRENT_CONTAINER_ID" ]; then
  echo "container ferrugem-web atual nao encontrado" >&2
  exit 1
fi

CURRENT_IMAGE_ID="$(docker inspect --format '{{.Image}}' "$CURRENT_CONTAINER_ID")"
docker image tag "$CURRENT_IMAGE_ID" "$ROLLBACK_IMAGE"

healthcheck() {
  docker compose exec -T caddy sh -lc "wget -qO- '$HEALTH_URL'" >/dev/null 2>&1
}

wait_for_health() {
  tries=1
  while [ "$tries" -le "$HEALTH_TRIES" ]; do
    if healthcheck; then
      return 0
    fi
    sleep "$HEALTH_SLEEP_SECONDS"
    tries=$((tries + 1))
  done
  return 1
}

echo "==> Backup pre-deploy"
./deploy/backup.sh

echo "==> Build da imagem de producao"
DOCKER_BUILDKIT=1 docker compose build ferrugem-web

echo "==> Atualizando servicos"
docker compose up -d ferrugem-web redis caddy

echo "==> Validando healthcheck da nova versao"
if ! wait_for_health; then
  echo "healthcheck falhou; iniciando rollback automatico" >&2
  docker image tag "$ROLLBACK_IMAGE" "$IMAGE_NAME"
  docker compose up -d ferrugem-web
  if ! wait_for_health; then
    echo "rollback falhou; verifique os logs do ferrugem-web e do caddy" >&2
    docker compose logs --tail=100 ferrugem-web caddy >&2 || true
    exit 1
  fi
  echo "rollback concluido com sucesso" >&2
  exit 1
fi

echo "==> Estado final"
docker compose ps
