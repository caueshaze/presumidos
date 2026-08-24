#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

IMAGE_NAME="presumidos/ferrugem-web:local-prod"
HEALTH_URL="http://ferrugem-web:8080/health/ready"
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

APP_CONTAINER="$(docker compose ps -q ferrugem-web 2>/dev/null || true)"
if [ -z "$APP_CONTAINER" ]; then
  echo "container ferrugem-web atual não encontrado; deploy.sh exige backup pre-deploy" >&2
  exit 1
fi

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

echo "==> Build da imagem de producao"
DOCKER_BUILDKIT=1 docker compose build ferrugem-web

echo "==> Backup pre-deploy"
PRESUMIDOS_APP_CONTAINER="$APP_CONTAINER" \
PRESUMIDOS_CLI_IMAGE="$IMAGE_NAME" ./deploy/backup.sh

echo "==> Parando o app para aplicar migrations offline"
docker compose stop ferrugem-web

echo "==> Aplicando migrations na imagem nova"
PRESUMIDOS_APP_CONTAINER="$APP_CONTAINER" \
PRESUMIDOS_CLI_IMAGE="$IMAGE_NAME" ./deploy/run-cli.sh migrate

echo "==> Atualizando servicos"
docker compose up -d ferrugem-web redis caddy

echo "==> Validando healthcheck da nova versao"
if ! wait_for_health; then
  echo "healthcheck falhou; nao fazer rollback de binario apos migration" >&2
  echo "valide logs/readiness e use restore de backup + versao compativel se necessario" >&2
  docker compose logs --tail=100 ferrugem-web caddy >&2 || true
  exit 1
fi

echo "==> Estado final"
docker compose ps
