#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

if [ "${1:-}" = "" ]; then
  echo "Uso: deploy/restore.sh backups/ferrugem-YYYYMMDD-HHMMSS.db" >&2
  exit 1
fi

BACKUP="$1"

[ -f "$BACKUP" ] || { echo "arquivo nao encontrado: $BACKUP" >&2; exit 1; }

case "$BACKUP" in
  backups/*) ;;
  *) echo "o backup precisa estar dentro de ./backups" >&2; exit 1 ;;
esac

NAME=$(basename "$BACKUP")

echo "Validando integridade de $BACKUP..."
RESULT=$(docker compose --profile tools run --rm -T sqlite-tool -c "sqlite3 /backups/$NAME 'PRAGMA integrity_check;'")
if [ "$RESULT" != "ok" ]; then
  echo "integrity_check falhou para $BACKUP: $RESULT" >&2
  exit 1
fi

echo "Isso vai PARAR o app e substituir o banco de producao por $BACKUP."
printf "Digite 'sim' para confirmar: "
read -r CONFIRM
[ "$CONFIRM" = "sim" ] || { echo "abortado"; exit 1; }

echo "Criando backup pre-restore do estado atual..."
mkdir -p backups
chmod 700 backups
docker compose --profile tools run --rm -T \
  -e "BACKUP_UID=$(id -u)" -e "BACKUP_GID=$(id -g)" \
  backup

echo "Parando a aplicacao..."
docker compose down

echo "Restaurando $BACKUP..."
docker compose --profile tools run --rm -T sqlite-tool -c \
  "rm -f /data/bolao.db /data/bolao.db-wal /data/bolao.db-shm && cp /backups/$NAME /data/bolao.db"

echo "Subindo a aplicacao..."
docker compose up -d

echo "Restore concluido."
