#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

if [ "${1:-}" = "" ]; then
  echo "Uso: infra/restore.sh backups/backup-YYYYMMDDTHHMMSSZ-XXXXXXXX" >&2
  exit 1
fi

BACKUP="$1"
[ -d "$BACKUP" ] || { echo "diretorio de backup nao encontrado: $BACKUP" >&2; exit 1; }
case "$BACKUP" in
  backups/*) ;;
  *) echo "o backup precisa estar dentro de ./backups" >&2; exit 1 ;;
esac

NAME=$(basename "$BACKUP")
echo "Validando $NAME..."
docker compose run --rm --no-deps --user 0 --entrypoint /app/ferrugem-web ferrugem-web \
  backup verify "/backups/$NAME"

echo "Este procedimento PARA a aplicacao e substitui o estado ativo."
printf "Digite 'sim' para confirmar: "
read -r CONFIRM
[ "$CONFIRM" = "sim" ] || { echo "abortado"; exit 1; }

echo "Criando backup pre-restore..."
./infra/backup.sh

echo "Parando servicos..."
docker compose down

echo "Restaurando em staging e ativando atomicamente..."
docker compose run --rm --no-deps --user 0 --entrypoint /app/ferrugem-web ferrugem-web \
  backup restore \
  --input "/backups/$NAME" \
  --database /data/bolao.db \
  --assets /data/assets \
  --replace

echo "Subindo servicos..."
docker compose up -d
echo "Restore concluido; aguarde e valide /health/ready."
