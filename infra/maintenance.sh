#!/bin/sh
# Liga/desliga a tela de manutenção servida pelo Caddy.
# Efeito imediato: nao precisa de reload nem rebuild.
#
#   ./infra/maintenance.sh on     # mostra a tela de "Intervalo" (HTTP 503)
#   ./infra/maintenance.sh off    # volta o app ao normal
#   ./infra/maintenance.sh status # mostra o estado atual
set -eu

cd "$(dirname "$0")/.."
FLAG="infra/maintenance/maintenance.flag"

case "${1:-}" in
  on)
    touch "$FLAG"
    echo "manutencao LIGADA -> o site mostra a tela de intervalo (503)"
    ;;
  off)
    rm -f "$FLAG"
    echo "manutencao DESLIGADA -> o app voltou ao normal"
    ;;
  status)
    if [ -f "$FLAG" ]; then
      echo "manutencao: LIGADA"
    else
      echo "manutencao: desligada"
    fi
    ;;
  *)
    echo "uso: $0 {on|off|status}" >&2
    exit 1
    ;;
esac
