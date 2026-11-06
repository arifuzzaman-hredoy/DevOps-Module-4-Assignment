# Nexora

Nexora is a task manager with a React web client and a Rust/Axum REST API backed by PostgreSQL. The repository includes a local Docker Compose stack and configuration/documentation for a manually operated three-tier deployment. No AWS infrastructure is provisioned by this project.

## Features

- User registration and login with server-side sessions and CSRF protection.
- Owner-scoped task CRUD, pagination, status filtering, and partial updates.
- Unicode-aware title validation and consistent JSON API errors.
- Responsive frontend with accessible forms and task management.
- PostgreSQL migrations, readiness checks, and persistent local Compose storage.

## Architecture

```text
Browser → Nginx → React static site
                 └→ /api → Axum API → PostgreSQL
```

In the intended deployment, Nginx is on the public Web tier, the API is on a private App tier, and PostgreSQL is on a private Database tier. See [Architecture](docs/ARCHITECTURE.md).

## Technology

- Backend: Rust, Axum 0.8, Tokio, SQLx, PostgreSQL 16
- Frontend: React, TypeScript, Vite, Vitest, Testing Library
- Local stack: Docker Compose and Nginx

## Project Structure

```text
backend/       Rust API, tests, and authoritative SQLx migrations
database/      Schema reference and optional local seed data
docs/          Development, architecture, deployment, testing, and API guides
frontend/      React/TypeScript application and tests
nginx/         Local proxy and manual deployment templates
screenshots/   Assignment evidence directory (currently empty)
scripts/       Build, test, health, local stack, and database utility scripts
docker-compose.yml
```

## Quick Start

Prerequisites: Docker Engine/Desktop with Compose. Create `.env.compose.local` from `.env.compose.example` if it does not already exist, set a local-only `DB_PASSWORD`, then start the services:

```bash
if [ ! -f .env.compose.local ]; then cp .env.compose.example .env.compose.local; fi
# Edit .env.compose.local locally; do not overwrite an existing file.
docker compose --env-file .env.compose.local config --quiet
docker compose --env-file .env.compose.local up --build -d --wait
```

If reusing the existing `nexora-local_nexora-postgres-data` volume, use the PostgreSQL password already associated with that data. Normal Compose stops preserve the volume; do not use `down -v` to resolve credential or migration issues.

Open [http://127.0.0.1:8081](http://127.0.0.1:8081). The UI uses same-origin `/api/` requests. See [Development](docs/DEVELOPMENT.md) for environment settings and native development.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Development and local setup](docs/DEVELOPMENT.md)
- [Deployment handoff](docs/DEPLOYMENT.md) — instructions/templates only; not an AWS deployment
- [Testing and verified results](docs/TESTING.md)
- [API reference](docs/API_DOCUMENTATION.md)
- [Database and migration notes](docs/DATABASE.md)
- [Security notes](docs/SECURITY.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
