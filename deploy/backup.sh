#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

mkdir -p backups
chmod 700 backups

./deploy/run-cli.sh backup create --output /backups
