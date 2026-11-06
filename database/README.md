# Nexora Database

SQLx migrations in `backend/migrations/` are the only authoritative schema change mechanism. The backend applies pending migrations at startup.

- [`schema.sql`](./schema.sql) is a reference snapshot of the final schema; do not apply it in addition to migrations.
- [`seed.sql`](./seed.sql) is optional sample data for local development only. It is not run by Compose or by the backend.
- `docker-compose.yml` runs PostgreSQL 16, the backend, frontend, and Nginx. PostgreSQL is bound to loopback and persists data in `nexora-local_nexora-postgres-data`.

The authoritative migration history is `backend/migrations/`; it includes task creation, the stable `(created_at, id)` listing index, and authentication/task ownership. See [database and migration notes](../docs/DATABASE.md) before applying changes to existing data.

Do not run local test or seed operations against AWS or any production database. The seed data is optional and must only be used in a disposable local environment.
