#!/bin/sh
set -eu

DB=/data/bolao.db
DEST=/backups

test -f "$DB" || { echo "Banco nao encontrado: $DB" >&2; exit 1; }

TS=$(date -u +%Y%m%d-%H%M%S)
FILE="$DEST/ferrugem-$TS.db"

sqlite3 "$DB" ".backup '$FILE'"

RESULT=$(sqlite3 "$FILE" "PRAGMA integrity_check;")
if [ "$RESULT" != "ok" ]; then
  echo "integrity_check falhou para $FILE: $RESULT" >&2
  rm -f "$FILE"
  exit 1
fi

if [ -n "${BACKUP_UID:-}" ] && [ -n "${BACKUP_GID:-}" ]; then
  chown "$BACKUP_UID:$BACKUP_GID" "$FILE"
fi
chmod 600 "$FILE"

find "$DEST" -maxdepth 1 -name 'ferrugem-*.db' -mtime +14 -print -delete

echo "backup criado: $FILE"
