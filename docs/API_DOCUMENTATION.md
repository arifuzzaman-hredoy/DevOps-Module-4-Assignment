# API Documentation

Base path: `/api`. All API error bodies use `{"error":{"code":"...","message":"..."}}`. Account endpoints use a server-side session cookie; all task endpoints require an authenticated session and operate only on that user's tasks.

In the local Docker stack, use the same-origin base `http://127.0.0.1:8081`; Nginx forwards `/api/` requests to the backend. The frontend sends cookies with same-origin credentials and keeps the CSRF token in memory.

## Authentication and Sessions

### `POST /api/auth/register`

Creates an account and signs it in, returning `201 Created` with a user profile and setting an HttpOnly session cookie. Body:

```json
{"email":"person@example.com","password":"StrongPassword123!"}
```

Email is trimmed, normalized to lowercase, and validated. Passwords must be 6–128 bytes and include uppercase and lowercase ASCII letters, a digit, and a symbol. Invalid input returns `400`; a duplicate normalized email returns `409`.

### `POST /api/auth/login`

Body has the same `email` and `password` fields. On success returns `200 OK` with the profile and sets a newly rotated session cookie. Invalid credentials return a generic `401`; login requests from one client IP are limited to 10 attempts per 15-minute window per application process and return `429` while limited. A successful login clears that IP's history.

### `GET /api/auth/me`

Returns the current user's profile (`id`, `email`, and `created_at`). Requires a valid session cookie; otherwise returns `401`.

### `GET /api/auth/csrf`

Returns `{"csrf_token":"..."}` for the authenticated session. The frontend keeps this value in memory and sends it as `X-CSRF-Token` on protected state-changing requests.

### `POST /api/auth/logout`

Requires a valid session and CSRF header, revokes the current server-side session, and clears the cookie. Returns `{"logged_out":true}`.

Session cookies are `HttpOnly`, `SameSite=Lax`, and scoped to `/`. `Secure` is enabled by default and can only be disabled with `APP_ENV=development`. Sessions expire after `SESSION_TTL_HOURS` (default 24, maximum 720).

## Health and Readiness

### `GET /health`

Returns `200 OK` when the process is running:

```json
{"status":"healthy","service":"nexora-api"}
```

### `GET /ready`

Returns the same shape with `"status":"ready"` and `200 OK` when PostgreSQL responds; otherwise returns `503 Service Unavailable` with the standard error envelope.

## Task Schema

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Learn Docker",
  "description": "Complete Docker fundamentals",
  "completed": false,
  "created_at": "2026-01-01T10:00:00Z",
  "updated_at": "2026-01-01T10:00:00Z"
}
```

Titles are trimmed and must contain 1–200 Unicode scalar values. Description can be omitted or null on creation. A task is visible only to its authenticated owner; another user's task ID is treated as not found.

## `POST /api/tasks`

Create a task:

```json
{"title":"Learn Docker","description":"Complete Docker fundamentals"}
```

Requires authentication and a valid `X-CSRF-Token`. Returns `201 Created` with the created task. Invalid JSON, missing title, empty/whitespace title, or overlong title returns `400 Bad Request`.

## `GET /api/tasks`

Requires authentication. Returns only the current user's tasks with stable descending `(created_at, id)` ordering. Query parameters:

| Parameter | Default | Constraints |
|---|---:|---|
| `page` | 1 | integer ≥ 1 |
| `limit` | 10 | integer 1–100 |
| `completed` | omitted | `true` or `false`; filters by completion state |

Example: `GET /api/tasks?page=2&limit=20&completed=false`

Response `200 OK`:

```json
{
  "data": [],
  "pagination": {"page": 2, "limit": 20, "total": 0, "total_pages": 0}
}
```

Invalid, zero, or out-of-range pagination parameters return `400 Bad Request`. `total` is the count after the optional status filter. The frontend text search is local to the currently loaded page.

## `GET /api/tasks/{id}`

Requires authentication. Returns an owned task with `200 OK`, `404 Not Found` when the task does not exist or is owned by someone else, or `400 Bad Request` for an invalid UUID.

## `PUT /api/tasks/{id}`

Requires authentication and a valid `X-CSRF-Token`. Partial update; send any combination of fields:

```json
{"title":"Learn Advanced Docker","description":"Compose","completed":false}
```

- Omitted fields are left unchanged.
- `description: null` clears the description.
- A string `description` replaces the description.
- `completed: false` is a valid explicit update.
- A supplied title is trimmed and validated against the 200-character limit.

Returns `200 OK` with the updated task, `404 Not Found` if missing or not owned by the caller, or `400 Bad Request` for invalid UUID/body/fields.

## `DELETE /api/tasks/{id}`

Requires authentication and a valid `X-CSRF-Token`. Returns `204 No Content` when deleted; `404 Not Found` if missing or not owned by the caller; `400 Bad Request` for invalid UUID.

## Other Errors

- Unknown route: `404` with `ROUTE_NOT_FOUND`.
- Unsupported method on a known route: `405` with `METHOD_NOT_ALLOWED`.
- Missing/expired session: `401`; invalid CSRF token: `403`.
- Login rate limit: `429`.
- JSON body over 64 KiB: `413` with `PAYLOAD_TOO_LARGE`.
- Unexpected database failure: `500` generic error; readiness failure: `503`.

Example:

```json
{"error":{"code":"TASK_NOT_FOUND","message":"Task not found"}}
```

The login limiter is in-memory and per process; it is not a shared distributed limit across replicas and resets on restart. When deployed behind Nginx, set `TRUST_PROXY_HEADERS=true` only when the backend is network-restricted to that trusted proxy and Nginx overwrites `X-Real-IP` with the connecting client's address. The default is `false`, in which case the peer address is used.
