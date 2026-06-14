#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

if [ ! -f ".env" ]; then
  echo "arquivo .env nao encontrado na raiz do projeto" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker nao encontrado no PATH" >&2
  exit 1
fi

echo "==> Backup pre-deploy"
./deploy/backup.sh

echo "==> Build da imagem de producao"
DOCKER_BUILDKIT=1 docker compose build ferrugem-web

echo "==> Atualizando servicos"
docker compose up -d ferrugem-web redis caddy

echo "==> Estado final"
docker compose ps
