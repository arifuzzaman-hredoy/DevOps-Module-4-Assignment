# Nexora Architecture

## Three-Tier Architecture

1. **Web tier:** Public Nginx serves the static Vite build and proxies API requests to the private App tier.
2. **App tier:** Private Rust/Axum service listens on port 3000, validates requests, executes parameterized SQL, and exposes liveness/readiness routes.
3. **Database tier:** Private PostgreSQL 16 service listens on port 5432 and stores application data.

## Request Flow

```text
Browser
  ├─ GET /, static assets ───────────────> Nginx → frontend container
  └─ /api/*, /health, /ready ───────────> Nginx → Rust :3000 → PgPool → PostgreSQL :5432
```

Locally, Compose runs PostgreSQL, the Rust backend, a frontend container serving the static React build, and a separate Nginx reverse proxy. Only Nginx and an optional loopback DB port are published to the host. Docker networking uses service DNS (`postgres`, `backend`, `frontend`). Nginx publishes on `127.0.0.1:8081`; native Vite development proxies relative API routes to `127.0.0.1:3000`. Production browser code contains no EC2 address or credential.

## Boundaries and Trust

- Only Nginx is intended to receive public web traffic.
- The App tier should accept port 3000 only from the Web security group and explicitly approved operational sources.
- PostgreSQL should accept port 5432 only from the App security group and approved administration paths.
- Those AWS security-group and routing rules are manual and have not been configured or verified by this project.
- Database TLS defaults to `verify-full` and accepts a custom CA path. Local Compose uses an explicitly separate `disable` setting.
- Registration/login issue opaque server-side sessions; authenticated task operations are scoped by the session's user ID and state-changing requests require CSRF tokens.
- `TRUST_PROXY_HEADERS` defaults to false. Enable it only when the app tier is private and Nginx overwrites `X-Real-IP`; the login limiter otherwise uses the direct peer IP.
- The login rate limiter is process-local and is not shared across replicas. Use a shared/edge limit if scaling beyond one process.
- The Compose database uses the named volume `nexora-local_nexora-postgres-data`; normal stops and rebuilds preserve it. Migration `0003` halts on legacy tasks to prevent silent ownership assignment.

## AWS Assignment Status

The three-tier layout and intended network boundaries are documented, but this repository does not provision AWS infrastructure. VPC/subnets, route tables, Internet/NAT gateways, security groups, EC2 instances, SSH access, DNS, and production TLS have not been configured or verified in AWS. Local Compose networking is not evidence that AWS resources exist.

## Application Structure

- `backend/src/main.rs`: initialization, routes, request limit, and graceful shutdown.
- `backend/src/handlers.rs`: request/response orchestration.
- `backend/src/models.rs`: API types, patch semantics, validation, and pagination types.
- `backend/src/repository.rs`: PostgreSQL queries.
- `backend/src/errors.rs`: JSON API errors.
- `backend/src/db.rs`: environment-based pool and PostgreSQL TLS.
- `backend/src/auth.rs`, `backend/src/auth_handlers.rs`, and `backend/src/state.rs`: session, CSRF, password, and authentication behavior.
- `backend/migrations/`: authoritative ordered schema migrations.

The user-ownership migration stops if legacy task rows exist. See [Database and migration notes](DATABASE.md) before applying it to an existing database.

## Availability

`/health` is process liveness and does not require the database. `/ready` performs a lightweight database query. The server waits for SQLx migrations before it begins listening and drains requests on Ctrl-C/SIGTERM.
