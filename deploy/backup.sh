#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p backups
chmod 700 backups

docker compose --profile tools run --rm -T \
  -e "BACKUP_UID=$(id -u)" -e "BACKUP_GID=$(id -g)" \
  backup
