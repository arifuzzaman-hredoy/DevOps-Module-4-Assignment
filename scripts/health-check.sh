#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${HEALTH_BASE_URL:-http://127.0.0.1:3000}"
curl --fail --silent --show-error "$BASE_URL/health"
printf '\n'
curl --fail --silent --show-error "$BASE_URL/ready"
printf '\n'
