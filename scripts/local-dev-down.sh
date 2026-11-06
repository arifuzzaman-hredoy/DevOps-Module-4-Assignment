#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="$ROOT_DIR/.env.compose.local"
if [[ -f "$ENV_FILE" ]]; then
    docker compose --file "$ROOT_DIR/docker-compose.yml" --env-file "$ENV_FILE" stop
else
    DB_PASSWORD="${DB_PASSWORD:-unused-for-stop}" \
        docker compose --file "$ROOT_DIR/docker-compose.yml" stop
fi
printf 'Nexora services stopped. Its named PostgreSQL data volume was preserved.\n'
