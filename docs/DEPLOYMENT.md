# Manual Deployment Handoff

This document describes a possible deployment to the existing three-tier AWS architecture. It is an operator guide only: no AWS resources, servers, DNS, certificates, or security groups have been accessed or changed, and no deployment has been performed. Validate all values, commands, and procedures against the target environment before use.

## Assignment Readiness Checklist

| Requirement | Status |
|---|---|
| Web, App, and Database tier responsibilities | Documented target design; local app tiers run in Compose |
| Public Web EC2, private App EC2, private Database EC2 | Not provisioned or verified |
| VPC and public/private subnet separation | Not configured or verified |
| Route tables, Internet Gateway, and NAT Gateway | Not configured or verified |
| Security groups and tier-to-tier rules | Intended relationships documented; AWS rules not configured or verified |
| SSH access | Not configured or accessed |
| Nginx `/api/` reverse proxy | Local proxy configured and verified; AWS proxy not deployed |
| Backend, PostgreSQL, frontend integration | Locally implemented and verified |
| SQLx migrations | Locally executed; not applied to AWS |
| AWS deployment, DNS, and HTTPS | Not completed or verified |
| Assignment screenshots/evidence | `screenshots/` is present but empty; no AWS evidence is available in this repository |

This status covers the requirements supplied for this review. A separate original assignment rubric was not found in the repository; confirm any additional rubric-specific deliverables before submission.

## Target Architecture and Network

- **Web EC2 (public):** Nginx serves the built React assets and proxies `/api/` to the private App EC2.
- **App EC2 (private):** Rust/Axum listens on port 3000 and connects to PostgreSQL over port 5432.
- **Database EC2 (private):** PostgreSQL 16 accepts connections only from the App tier and approved administration paths.

Configure security groups so the Web tier receives only intended public HTTP/HTTPS traffic, the App tier accepts port 3000 only from the Web security group (plus explicitly approved operations), and the Database tier accepts port 5432 only from the App security group. Do not expose app or database ports publicly. The frontend uses same-origin `/api/` routes and needs no public backend address or API secret.

## Database Tier

Install and harden PostgreSQL 16 according to the operator's standards. Create the application role interactively so its password is not embedded in shell history:

```bash
sudo -u postgres psql
```

```sql
CREATE ROLE nexora_app LOGIN;
\password nexora_app
CREATE DATABASE nexora OWNER nexora_app;
\q
```

Configure PostgreSQL TLS using a trusted CA and a server certificate whose SAN matches the private DNS hostname used by the app. Restrict PostgreSQL listening addresses and `pg_hba.conf` to approved sources. Rehearse schema changes and backup/restore on a restored copy first; see [Database and migration notes](DATABASE.md).

## App Tier

Build the Linux release binary for the target OS/architecture from a reviewed checkout:

```bash
cd backend
cargo build --locked --release
```

Install the binary and migrations in a protected application directory, or build/deploy only the backend image from `backend/Dockerfile`. Do not deploy the local PostgreSQL, frontend, or public Nginx Compose services on the App EC2.

Supply settings through an approved secret-management mechanism, not a repository `.env` or image layer. Typical production settings:

```text
DB_HOST=<private-db-dns-name-matching-certificate>
DB_PORT=5432
DB_NAME=nexora
DB_USER=nexora_app
DB_PASSWORD=<supplied-out-of-band>
DB_SSL_MODE=verify-full
DB_SSL_ROOT_CERT=/etc/nexora/certs/postgresql-ca.pem
APP_ENV=production
SESSION_COOKIE_SECURE=true
SESSION_TTL_HOURS=24
TRUST_PROXY_HEADERS=true
RUST_LOG=info
```

Install the verified CA read-only. Keep port 3000 private. Enable `TRUST_PROXY_HEADERS=true` only when the App tier accepts traffic exclusively from the trusted Web proxy and that proxy overwrites `X-Real-IP`. The built-in login limiter is per-process; use shared or edge rate limiting when needed for multiple instances.

For a systemd deployment, review and adapt a unit with a dedicated unprivileged user, protected environment file, restart policy, and filesystem hardening. Do not install/reload services until the binary, secret delivery, database connectivity, and migration plan have been reviewed. Backend startup applies pending SQLx migrations before accepting requests.

## Web Tier, Nginx, DNS, and HTTPS

Build the frontend in `frontend/`:

```bash
npm ci
npm run typecheck
npm run lint
npm test
npm run build
```

Copy only `frontend/dist/` to `/var/www/nexora/` on the Web EC2. Install Nginx and `envsubst` using the host's approved package source. Choose `nginx/nexora-http.conf.template` only if TLS terminates elsewhere; otherwise use `nginx/nexora-https.conf.template` with externally managed certificates and renewal.

From the project root, render the HTTP template with only its intended variables:

```bash
export NEXORA_DOMAIN='your-domain.example'
export NEXORA_APP_HOST='private-app-hostname'
envsubst '${NEXORA_DOMAIN} ${NEXORA_APP_HOST}' \
  < nginx/nexora-http.conf.template \
  | sudo tee /etc/nginx/sites-available/nexora.conf >/dev/null
```

For the HTTPS template, also set `NEXORA_TLS_CERT` and `NEXORA_TLS_KEY`, and pass all four placeholders explicitly to `envsubst`. Do not run an unrestricted `envsubst`; it would replace Nginx's `$host`, `$remote_addr`, `$scheme`, and `$request_uri` variables.

Review the rendered file, enable it according to the host's Nginx layout, and run `sudo nginx -t` before an operator-controlled reload. Point DNS to the Web tier only after the Web configuration and certificate are ready. No production Nginx template, DNS, or certificate has been validated by this project.

The templates route `/api/` to `${NEXORA_APP_HOST}:3000`, set forwarding headers, support SPA fallback, and apply basic security headers. They cap request bodies at 1 MiB; the backend JSON limit is 64 KiB.

## Rollout, Health, and Rollback

1. Review the release, dependencies, configuration, migration list, and security-group plan.
2. Take and verify a restorable PostgreSQL backup. Migration `0003` stops if legacy tasks exist; resolve that rollout blocker using the reviewed ownership-migration plan in [Database notes](DATABASE.md). Never silently assign or discard tasks.
3. Roll out the Database, App, and Web changes using the organization's approved change process.
4. On the App host, verify `http://127.0.0.1:3000/health` and `/ready`. From an approved client, verify the HTTPS Web URL and exercise registration, login/logout, and disposable task CRUD, including ownership boundaries.
5. Preserve previous application/frontend artifacts and a verified database recovery path. Roll back application artifacts separately from schema. Migrations are forward-only; never delete the database or volume as a rollback strategy.

Deployment, SSH/transfer, AWS configuration, production tests, backup scheduling, TLS issuance, and rollback have not been performed or verified here. See [Troubleshooting](TROUBLESHOOTING.md) for local-only Compose troubleshooting.
