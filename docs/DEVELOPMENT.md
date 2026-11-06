# Development

## Prerequisites

- Docker Engine/Desktop with Docker Compose for the complete local stack.
- Rust toolchain compatible with the project and PostgreSQL client tools for native backend work or database utilities.
- Node.js/npm for frontend development.
- Bash/WSL for the helper scripts; the Compose commands can also be run from PowerShell.

## Environment Configuration

The Compose stack reads `.env.compose.local`, which is ignored by Git. Create it from the non-secret example only when it does not already exist:

```bash
if [ ! -f .env.compose.local ]; then cp .env.compose.example .env.compose.local; fi
```

Set `DB_PASSWORD` locally. For an existing `nexora-local_nexora-postgres-data` volume, use the password that initialized that database; changing the Compose variable does not rotate a PostgreSQL role password. Do not overwrite an existing environment file or put credentials in source control.

| Variable | Required | Purpose |
|---|---:|---|
| `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASSWORD` | Yes | Backend PostgreSQL connection |
| `DB_SSL_MODE` | No | PostgreSQL TLS mode; production default is `verify-full`, local Compose sets `disable` |
| `DB_SSL_ROOT_CERT` | No | Trusted CA path used for verified database TLS |
| `APP_ENV` | No | Defaults to production; `development` permits a non-Secure local cookie |
| `SESSION_COOKIE_NAME` | No | Session cookie name |
| `SESSION_COOKIE_SECURE` | No | Secure-cookie setting; disabling is restricted to development |
| `SESSION_TTL_HOURS` | No | Session lifetime, default 24 |
| `TRUST_PROXY_HEADERS` | No | Defaults false; enable only behind a trusted proxy that overwrites `X-Real-IP` |
| `RUST_LOG` | No | Backend tracing filter |
| `NEXORA_HTTP_HOST_PORT` | No | Loopback host port for Nginx; default 8081 |
| `NEXORA_DB_HOST_PORT` | No | Optional loopback PostgreSQL port; default 5432 |

Database backup/restore scripts additionally use `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, and a secure password mechanism such as `PGPASSFILE`; `BACKUP_DIR` can select the output directory. `HEALTH_BASE_URL` configures `scripts/health-check.sh`. Never place production secrets in command arguments or frontend build variables.

## Run the Full Local Stack

From the project root:

```bash
docker compose --env-file .env.compose.local config --quiet
docker compose --env-file .env.compose.local up --build -d --wait
docker compose --env-file .env.compose.local ps
```

Open `http://127.0.0.1:8081`. Nginx serves the frontend and proxies `/api/`, `/health`, and `/ready` to the backend. PostgreSQL is optionally published on loopback; backend port 3000 and frontend port 80 are private to the Compose network. SQLx migrations run before the backend accepts requests.

To stop services without removing persistent data:

```bash
docker compose --env-file .env.compose.local stop
```

Do not use `docker compose down -v`, delete the named volume, or use Docker prune commands. Migration `0003` intentionally stops if legacy task rows need ownership mapping; follow [Database notes](DATABASE.md) rather than resetting the database.

## Native Development

The native workflow runs the backend and Vite outside Compose; do not run it alongside the corresponding Compose backend on the same ports. Start a local PostgreSQL instance separately and configure the backend using the local-only template `backend/.env.local.example`. Copy it only when `backend/.env` does not already exist, and never replace an existing environment file:

```bash
if [ ! -f backend/.env ]; then cp backend/.env.local.example backend/.env; fi
(cd backend && cargo run)
```

In another terminal:

```bash
cd frontend
npm ci
npm run dev
```

Vite proxies API and health requests to `127.0.0.1:3000`. Do not reuse example credentials outside isolated local development.

## Useful Commands

From the project root, Bash/WSL helper scripts include:

```bash
./scripts/run-backend-tests.sh
./scripts/run-frontend-tests.sh
./scripts/build-backend.sh
./scripts/build-frontend.sh
./scripts/health-check.sh
```

The scripts `database-backup.sh` and `database-restore.sh` require PostgreSQL client tools and explicit operator-supplied connection settings. The restore helper does not drop existing objects, but can add or conflict with data; use only a reviewed target and backup.

See [Testing](TESTING.md) for test commands and actual verification results, and [Troubleshooting](TROUBLESHOOTING.md) for local stack issues.
