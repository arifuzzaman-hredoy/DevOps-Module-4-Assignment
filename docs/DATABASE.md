# Database and Migration Notes

## Schema Authority

`backend/migrations/` is the authoritative schema history. SQLx applies pending migrations at startup and records them in `_sqlx_migrations`. `database/schema.sql` is a reference snapshot only; do not apply it in addition to the migrations.

## Schema and Migration History

The task table contains `id` (UUID primary key), `title` (non-null, maximum 200 characters), nullable `description`, `completed` (non-null, default false), and non-null `created_at`/`updated_at` timestamps. Later migrations add stable pagination indexing, authentication tables, and required task ownership.

Migrations are append-only; do not edit an applied migration. Add a new migration for schema changes:

1. `0001_create_tasks.sql` creates the initial task table and creation-time index.
2. `0002_add_stable_task_pagination_index.sql` adds the composite ordering index.
3. `0003_add_authentication_and_task_ownership.sql` adds users/sessions and required ownership.

## Authentication and Ownership

- `users` stores normalized unique email addresses and Argon2id password hashes.
- `sessions` stores a SHA-256 hash of the random session token, its CSRF token, user ID, and expiry. The browser's raw session token is only in an HttpOnly cookie.
- `tasks.user_id` is mandatory and references `users.id` with `ON DELETE CASCADE`.
- Task list, count, fetch, update, and delete queries are scoped by the authenticated user ID. There is no client-supplied owner ID.

## Migration 0003 and Legacy Tasks

`0003_add_authentication_and_task_ownership.sql` stops with an explicit error if the existing `tasks` table contains rows. It does not silently assign legacy data to an arbitrary account. On PostgreSQL, this migration is transactional, so a failed guard does not leave a partially created schema.

For a database with legacy tasks, the current migration blocks startup; it does not provide an in-place ownership mapping. Treat that as a rollout blocker. Before deploying, stop writes, take and validate a restorable backup, define the intended owner mapping, and design a reviewed migration/release procedure that preserves every required row. Rehearse it on an isolated restored copy and verify counts, ownership, rollback, and restore before production approval. Do not start the new version against legacy rows, truncate or discard tasks as a shortcut, disable the guard, or invent a shared/default owner.

## Local Development Data

Compose runs PostgreSQL 16 locally with a persistent named volume and a health check. `database/seed.sql` is optional and expects a local account already registered with `demo@example.test`; it assigns sample rows only to that account. It is not run automatically and must never be applied to production.

## Backups

The scripts use `pg_dump` custom format and `pg_restore`. Supply `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, and a secure password mechanism such as `PGPASSFILE` through the operator environment. The restore script prompts before running and does not issue a drop/clean operation. Validate backup restoration separately before depending on it operationally.
