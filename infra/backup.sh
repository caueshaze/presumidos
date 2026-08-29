#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p backups
chmod 700 backups

./infra/run-cli.sh backup create --output /backups
