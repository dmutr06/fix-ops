# Stack

How these fit together: [architecture](architecture.md). Versions are the latest stable as of November 2026; minor versions float in `Cargo.lock`.

## Service

| Area | Choice | Why |
|---|---|---|
| Language | Rust 2024 edition | Strong types for money, roles and lifecycle states; one static binary |
| Async runtime | Tokio 1 | The default for axum and sqlx |
| HTTP | axum 0.8 | Thin, typed extractors; plain `tower` middleware |
| Middleware | tower-http 0.7 (`trace`) | Request logging without custom code |
| Database | PostgreSQL 18 | Transactions keep a change and its audit entry atomic; `pg_dump` covers backups |
| Database access | sqlx 0.9 (runtime queries, `FromRow`) | Async, plain SQL, embedded migrations, `#[sqlx::test]` creates an isolated database per test |
| Serialization | serde + serde_json | JSON DTOs in the HTTP layer only |
| Time | `time` 0.3 | RFC 3339 timestamps, maps to `timestamptz` |
| Password hashing | `argon2` | Current OWASP recommendation for stored passwords |
| Session tokens | `rand` + `sha2` | Random opaque bearer tokens; only their SHA-256 hash is stored |
| Errors | `thiserror` in libraries, `anyhow` in `main` | Typed errors where callers branch on them, context where they don't |
| Logging | `tracing` + `tracing-subscriber` | Structured logs, filtered by `RUST_LOG` |

## Tooling

| Area | Choice | Why |
|---|---|---|
| Local database | Docker Compose (`docker-compose.yml`) | Same Postgres version locally and in CI |
| Format and lint | `rustfmt`, `clippy` (`pedantic`, warnings fail CI) | One style, catches non-idiomatic code early |
| Tests | `cargo test`: unit tests inline, HTTP tests in `tests/` | HTTP tests run the real router in-process against a real database, no network |
| API contract | `spec/openapi.yaml`, checked by `tests/contract.rs`; Prism mock suite in `tests/prism` | The contract is the source of truth for routes, payloads and error codes |
| CI | GitHub Actions | fmt, clippy and tests against a Postgres service container on every PR |

## Conventions

- Dependencies are added with the feature that needs them, not upfront.
- Shared versions live in `[workspace.dependencies]`; crates use `dep.workspace = true`.
- SQL is written by hand with runtime-checked queries (`sqlx::query_as`), so building needs no database. Every query is covered by an HTTP test.

## Not used

| Tool | Reason |
|---|---|
| ORM (Diesel, SeaORM) | The schema is small; plain SQL is clearer and keeps transactions explicit |
| sqlx compile-time macros (`query!`) | They need a live database or an offline cache to build; HTTP tests cover the queries instead |
| `async-trait` | Ports use native `impl Future + Send` in traits |
| `validator` | Input rules live in domain constructors, so they are tested once and cannot be skipped |
| JWT | Server-side sessions can be revoked at once when an account is disabled (REQ-F-019) |
| Frontend framework | The API comes first; the staff web interface is planned separately |
