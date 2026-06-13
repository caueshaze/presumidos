#!/usr/bin/env bash
#
# Backup do banco SQLite de PRODUÇÃO antes de um deploy.
#
# O app roda em modo WAL, então NÃO se deve copiar o arquivo .db cru. Este script
# usa a API de backup *online* do SQLite (`.backup`), que é consistente mesmo com
# o app rodando e escrevendo — sem downtime. O `sqlite3` não precisa existir nem
# no host nem no container do app: usamos um container `alpine` descartável que
# monta o mesmo volume de dados.
#
# Uso:
#   scripts/backup-db.sh                 # backup + validação, saída em ./backups
#   OUTPUT_DIR=/mnt/bkp scripts/backup-db.sh
#
# Variáveis de ambiente (com defaults para a stack de produção atual):
#   VOLUME      Nome do volume Docker com o banco   (default: presumidos_app_data)
#   DB_PATH     Caminho do .db dentro do volume     (default: /data/bolao.db)
#   OUTPUT_DIR  Onde gravar o backup no host        (default: ./backups)
#   ALPINE_IMG  Imagem usada para rodar o sqlite3   (default: alpine)
#
set -euo pipefail

VOLUME="${VOLUME:-presumidos_app_data}"
DB_PATH="${DB_PATH:-/data/bolao.db}"
OUTPUT_DIR="${OUTPUT_DIR:-./backups}"
ALPINE_IMG="${ALPINE_IMG:-alpine}"

STAMP="$(date +%Y%m%d-%H%M%S)"
OUT="bolao-pre-deploy-${STAMP}.db"

mkdir -p "$OUTPUT_DIR"
ABS_OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"

echo "==> Volume:  $VOLUME"
echo "==> Banco:   $DB_PATH"
echo "==> Destino: $ABS_OUTPUT_DIR/$OUT"

# Backup online (consistente, com o app no ar). O alpine instala o sqlite3 só
# dentro do container efêmero. O volume é montado rw porque o SQLite precisa
# acessar os arquivos -wal/-shm para ler um banco em modo WAL; o `.backup` apenas
# LÊ a origem (/data) e ESCREVE a cópia em /backup — não altera o banco do app.
docker run --rm \
  -v "$VOLUME:/data" \
  -v "$ABS_OUTPUT_DIR:/backup" \
  "$ALPINE_IMG" sh -c \
  "apk add --no-cache sqlite >/dev/null 2>&1 && sqlite3 '$DB_PATH' \".backup /backup/$OUT\""

if [ ! -s "$ABS_OUTPUT_DIR/$OUT" ]; then
  echo "ERRO: backup não foi criado ou está vazio." >&2
  exit 1
fi

echo "==> Backup criado:"
ls -lh "$ABS_OUTPUT_DIR/$OUT"

# Validação de integridade + contagens básicas.
echo "==> Validando integridade..."
docker run --rm \
  -v "$ABS_OUTPUT_DIR:/backup" \
  "$ALPINE_IMG" sh -c \
  "apk add --no-cache sqlite >/dev/null 2>&1 && \
   sqlite3 /backup/$OUT 'PRAGMA integrity_check; SELECT \"pools=\" || count(*) FROM pools; SELECT \"users=\" || count(*) FROM users;'"

echo "==> OK. Guarde este arquivo fora do servidor antes do deploy, por exemplo:"
echo "    scp <user>@<host>:$ABS_OUTPUT_DIR/$OUT ./"
