#!/usr/bin/env bash
set -euo pipefail
umask 077

: "${PGHOST:?Set PGHOST}"
: "${PGPORT:?Set PGPORT}"
: "${PGDATABASE:?Set PGDATABASE}"
: "${PGUSER:?Set PGUSER}"
BACKUP_DIR="${BACKUP_DIR:-./backups}"
mkdir -p -- "$BACKUP_DIR"
OUTPUT="$BACKUP_DIR/${PGDATABASE}-$(date -u +%Y%m%dT%H%M%SZ).dump"

pg_dump --no-password --format=custom --file="$OUTPUT"
printf 'Database backup written to %s\n' "$OUTPUT"
