# Nexora Backend

Rust/Axum REST API backed by PostgreSQL through SQLx. The service listens on port 3000 and applies embedded SQLx migrations before accepting requests.

## Modules

- `src/main.rs`: initialization, routing, request limits, graceful shutdown.
- `src/db.rs`: connection pool and PostgreSQL TLS options.
- `src/models.rs`: request types, Unicode title validation, patch semantics, pagination.
- `src/handlers.rs`, `src/auth_handlers.rs`: HTTP handlers.
- `src/auth.rs`, `src/state.rs`: password hashing, sessions, CSRF, configuration, login limiting.
- `src/repository.rs`: parameterized PostgreSQL queries.
- `src/errors.rs`: standard JSON errors.
- `migrations/`: authoritative ordered schema migrations.

## Build and Test

Run from this directory:

```bash
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

Two database integration tests are ignored unless explicitly selected. Their safe setup, commands, and verified results are documented in [Testing](../docs/TESTING.md). Use separate disposable loopback databases; never use production or data that must be preserved.

## Development and API

See the root [Development guide](../docs/DEVELOPMENT.md) for local environment setup and [API documentation](../docs/API_DOCUMENTATION.md) for endpoints and request semantics.
