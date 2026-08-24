#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p backups
chmod 700 backups

docker compose exec -T --user 0 ferrugem-web \
  /app/ferrugem-web backup create --output /backups
