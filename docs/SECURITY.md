# Security Notes

## Implemented Safeguards

- Credentials are read from runtime environment variables; none are embedded in application source.
- Root and backend ignore rules exclude `.env` and secret variants while retaining example templates.
- SQL uses static statements and bound user parameters.
- Task titles are trimmed and bounded to 200 Unicode scalar values; UUIDs and pagination inputs are validated.
- API errors use generic public messages; SQLx errors are logged server-side rather than returned.
- Request JSON is limited to 64 KiB in the backend; Nginx templates use a 1 MiB outer cap.
- PostgreSQL TLS defaults to `verify-full`, with support for `DB_SSL_ROOT_CERT`. No silent TLS downgrade is configured by the default.
- The local Compose DB is bound to loopback, uses local-only credentials, and is not an AWS connection.

## Authentication and Authorization

- Registration and login use Argon2id password hashes and server-side sessions. Only a SHA-256 hash of each random session token is stored in PostgreSQL.
- Cookies are HttpOnly and SameSite=Lax; Secure is enabled by default and may be disabled only for `APP_ENV=development`.
- A per-session CSRF token is required on authenticated POST/PUT/PATCH/DELETE requests. The frontend keeps the token in memory and sends it in `X-CSRF-Token`.
- Every task query is scoped to the authenticated user. Client-supplied owner IDs are ignored, and non-owned tasks are indistinguishable from missing tasks.
- Login uses a generic credential error and a per-process IP limiter (10 attempts per 15 minutes; a successful login clears that IP's history). This limiter is not shared across replicas and resets on process restart; production deployments with multiple app instances should add a shared or edge rate limit.
- Account email verification, password reset/recovery, MFA, and registration abuse controls are not implemented. Define and operate an appropriate account recovery and abuse-response process before relying on this as a full production identity service.

The ownership migration intentionally stops if legacy task rows exist. Follow the reviewed rollout guidance in [Database and Migration Notes](DATABASE.md); do not remove the guard or silently assign old tasks.

## TLS Configuration

Production configuration should set `DB_SSL_MODE=verify-full` and install the correct trusted CA at `DB_SSL_ROOT_CERT`. Confirm the DB DNS name matches the certificate. Local Compose uses `DB_SSL_MODE=disable` because the local container has no TLS configured. No production certificate was created or verified.

## Network and Deployment Assumptions

Nginx is intended as the public entry point; application and database tiers should remain private. `TRUST_PROXY_HEADERS` defaults to false. Enable it only when the backend accepts connections exclusively from a trusted reverse proxy that overwrites `X-Real-IP`; otherwise a caller could spoof that header and circumvent per-IP rate limiting. The included Nginx templates set `X-Real-IP` from `$remote_addr`, but private networking/firewall controls must be configured and verified separately. AWS security groups, routing, SSH, host hardening, TLS issuance, service account permissions, log retention, backups, and monitoring are manual and unverified.

## Reporting

Do not log request bodies, authorization material, or environment contents. Review service logs and dependencies under the organization's operational policies. Review the current dependency lockfile and relevant security advisories before a release.
