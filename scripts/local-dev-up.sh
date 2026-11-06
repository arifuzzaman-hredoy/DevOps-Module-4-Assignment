#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="$ROOT_DIR/.env.compose.local"
if [[ ! -f "$ENV_FILE" ]]; then
    printf 'Create %s from .env.compose.example and set DB_PASSWORD first.\n' "$ENV_FILE" >&2
    exit 1
fi

docker compose \
    --file "$ROOT_DIR/docker-compose.yml" \
    --env-file "$ENV_FILE" \
    up --build -d --wait
printf 'Nexora is available at http://127.0.0.1:%s.\n' "${NEXORA_HTTP_HOST_PORT:-8081}"
printf 'PostgreSQL is available on 127.0.0.1 (port configured by NEXORA_DB_HOST_PORT).\n'
