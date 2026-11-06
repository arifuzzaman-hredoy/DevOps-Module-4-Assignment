#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 || ! -f "$1" ]]; then
  printf 'Usage: %s <custom-format-backup-file>\n' "$0" >&2
  exit 2
fi

: "${PGHOST:?Set PGHOST}"
: "${PGPORT:?Set PGPORT}"
: "${PGDATABASE:?Set PGDATABASE}"
: "${PGUSER:?Set PGUSER}"
printf 'This restores data into database "%s" at host "%s". Existing objects are not dropped, but restore can add or modify rows.\n' "$PGDATABASE" "$PGHOST"
read -r -p 'Continue? [y/N] ' confirmation
if [[ "$confirmation" != "y" && "$confirmation" != "Y" ]]; then
  printf 'Restore cancelled.\n'
  exit 1
fi

pg_restore --no-password --no-owner --no-acl --exit-on-error --dbname="$PGDATABASE" "$1"
