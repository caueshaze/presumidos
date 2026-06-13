#!/usr/bin/env bash
#
# Cria o admin inicial do ambiente de DEV via bootstrap.
#
# Roda sempre a partir da raiz do repositório, então o binário carrega o `.env`
# e o `bolao.db` corretos (DATABASE_PATH é relativo ao diretório de execução).
#
# Observações:
#   - O bootstrap só funciona enquanto NÃO existir nenhum admin no banco — é a
#     criação do *primeiro* admin. Para recomeçar do zero em dev, apague o banco:
#       rm -f bolao.db bolao.db-shm bolao.db-wal
#   - Defina BOOTSTRAP_ADMIN_PASSWORD para informar a senha sem prompt interativo;
#     caso contrário, a senha é pedida de forma interativa.
#
# Uso:
#   scripts/dev-admin.sh <username> <email>
#   BOOTSTRAP_ADMIN_PASSWORD=senha-forte scripts/dev-admin.sh admin admin@local.dev
#
set -euo pipefail

cd "$(dirname "$0")/.."

USERNAME="${1:-}"
EMAIL="${2:-}"
if [ -z "$USERNAME" ] || [ -z "$EMAIL" ]; then
  echo "uso: scripts/dev-admin.sh <username> <email>"
  echo "  (opcional: BOOTSTRAP_ADMIN_PASSWORD=... para senha não-interativa)"
  exit 1
fi

exec cargo run -p ferrugem-web -- bootstrap-admin --username "$USERNAME" --email "$EMAIL"
