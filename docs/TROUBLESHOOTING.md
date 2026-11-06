# Troubleshooting

Use the Compose-only environment file for every Docker Compose command:

```bash
docker compose --env-file .env.compose.local ps
docker compose --env-file .env.compose.local logs --tail=100 postgres backend frontend nginx
```

Do not paste environment values or secret-bearing logs into tickets or chat.
See [Development](DEVELOPMENT.md) for safe local setup and environment configuration.

## Compose says `DB_PASSWORD` is required

Copy `.env.compose.example` to `.env.compose.local`, set a local-only `DB_PASSWORD`, then retry. Do not edit or copy over `backend/.env`.

## PostgreSQL is healthy but the backend is not ready

Check backend logs for connection and migration errors. For an existing persistent volume, the `DB_PASSWORD` used by the backend must match the password that initialized the PostgreSQL data directory; changing `POSTGRES_PASSWORD` in the Compose environment does not rotate a password in an existing database.

If logs report that migration `0003` cannot add ownership while legacy tasks exist, stop and follow [Database migration notes](./DATABASE.md). The migration intentionally halts; do not reset the database or delete the volume.

## Host port is already in use

Set `NEXORA_HTTP_HOST_PORT` in `.env.compose.local` to an available local port (default `8081`) or `NEXORA_DB_HOST_PORT` (default `5432`), then recreate the relevant Nexora service with:

```bash
docker compose --env-file .env.compose.local up -d
```

Only Nexora services are managed by this project. Do not stop or remove unrelated containers.

## Frontend loads but API requests fail

Check that `backend` and `postgres` are healthy, then inspect the Nginx/backend logs. The browser uses same-origin `/api/` requests; backend port 3000 is intentionally not published to the host. A `401` means sign in again; a `403` indicates the session CSRF token is missing or stale, so refresh and retry.

## Docker daemon or build failures

Start Docker Desktop/Engine and verify `docker info`. Builds require access to the configured container registries and, for the frontend/backend build stages, package registries. No AWS resources are involved.
