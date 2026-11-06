# Testing

## Recorded Verification Results — 2026-11-06

| Area | Result |
|---|---|
| Backend format | `cargo fmt --check` — PASS |
| Backend compile | `cargo check` — PASS |
| Backend lint | `cargo clippy -- -D warnings` — PASS |
| Standard backend suite | `cargo test` — PASS, 22 passed, 2 ignored |
| PostgreSQL CRUD integration | `local_postgres_crud_and_partial_updates` — PASS, 1 passed, 0 failed |
| PostgreSQL auth integration | `authentication_csrf_and_task_ownership_flow` — PASS, 1 passed, 0 failed |
| Frontend TypeScript | PASS |
| Frontend ESLint | PASS |
| Frontend Vitest | PASS, 29 tests passed |
| Frontend production build | PASS |
| Docker Compose configuration | PASS |
| Local Nginx configuration | PASS (`nginx -t`) |
| Bash helper scripts | PASS (`bash -n`, all 9 scripts) |
| Local Docker | Four services healthy; Nginx routing, auth, and task CRUD verified |

Integration test results are from separate runs against disposable loopback PostgreSQL databases. In the standard suite those tests remain ignored unless explicitly selected. These local results do not imply AWS or production verification.

## Backend

From `backend/`:

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

Unit tests cover Unicode title validation and boundaries, patch-field deserialization, `completed: false`, pagination defaults/range validation, authentication helpers, and proxy-IP trust behavior.

The PostgreSQL-backed router test additionally exercises process health and DB readiness, migration repeat-startup, registration, login/logout/session revocation, authentication failures, CSRF rejection, task CRUD and ownership isolation, invalid JSON/UUID/title requests, task title and user foreign-key constraints, and session persistence/expiry.

The repository integration test covers create/list/count/get/update/delete, omitted and null patch fields, explicit `completed: false`, and `updated_at` changes. It creates a temporary table on its dedicated database connection.

Two PostgreSQL integration tests are ignored by default:

- `local_postgres_crud_and_partial_updates` uses a temporary table on a dedicated connection and requires a loopback URL in `NEXORA_TEST_DATABASE_URL`.
- `authentication_csrf_and_task_ownership_flow` applies migrations and requires a disposable loopback database in `NEXORA_AUTH_TEST_DATABASE_URL`. It stops if legacy task rows exist.

Run each separately against its own disposable loopback PostgreSQL database:

```bash
NEXORA_TEST_DATABASE_URL='<disposable-loopback-postgres-url>' \
  cargo test local_postgres_crud_and_partial_updates -- --ignored

NEXORA_AUTH_TEST_DATABASE_URL='<separate-disposable-loopback-postgres-url>' \
  cargo test authentication_csrf_and_task_ownership_flow -- --ignored
```

Both ignored tests were executed separately against their configured disposable loopback PostgreSQL databases:

- `local_postgres_crud_and_partial_updates`: **1 passed, 0 failed**.
- `authentication_csrf_and_task_ownership_flow`: **1 passed, 0 failed**.

The standard `cargo test` run reports **22 passed, 2 ignored**; the ignored tests require explicit database configuration. The separate integration runs each reported **1 passed, 0 failed**. Never run them against AWS or a database containing data to preserve.

## Frontend

From `frontend/`:

```bash
npm ci
npm run typecheck
npm run lint
npm test
npm run build
```

Vitest/Testing Library tests cover auth route/form behavior, dashboard rendering, task listing, loading/error/empty states, task creation and editing, completion, filtering, search, delete confirmation, dialog Escape/focus behavior, and API client paths/error conversion.

## Compose

```bash
docker compose --env-file .env.compose.local config --quiet
docker compose --env-file .env.compose.local build
docker compose --env-file .env.compose.local up -d --wait
docker compose --env-file .env.compose.local ps
curl --fail http://127.0.0.1:8081/health
curl --fail http://127.0.0.1:8081/ready
curl --fail http://127.0.0.1:8081/
```

Build and runtime verification use the existing named `nexora-local_nexora-postgres-data` volume and preserve it. Stop with `scripts/local-dev-down.sh`; do not use `down -v`.

## Local Docker Verification

The local stack was built and started with `docker compose up --build -d --wait --wait-timeout 240`. PostgreSQL, backend, frontend, and Nginx all reported healthy. Backend startup migrations completed against the existing local database. The named PostgreSQL volume remained `nexora-local_nexora-postgres-data`. Local verification was performed on 2026-11-06; these results are separate from AWS/production testing.

The following checks passed through the published local Nginx endpoint at `http://127.0.0.1:8081`:

- Compose configuration validation and both application image builds.
- `bash -n` validation of all nine shell scripts.
- `nginx -t` inside the running local Nginx container.
- Frontend root and SPA `/login` responses, plus backend `/health` and `/ready`.
- Registration, login, current-user lookup, and logout.
- Task create, list, get-by-ID, update, and delete.
- Partial update with `completed: false` and `description: null`.
- Invalid task UUID returns HTTP 400.

The smoke tests used new local accounts and deleted their test tasks. No production service or database was used.

## Coverage Gaps

- No automated real-browser end-to-end test was run. HTTP smoke tests exercised the running stack through Nginx.
- AWS connectivity, AWS deployment templates, and production deployment are explicitly not tested.

Report PASS only for commands actually executed. Environment-specific toolchain deviations should be described alongside results.
