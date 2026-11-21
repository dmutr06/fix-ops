# FixOps

Internal service for a PC and electronics repair workshop: clients, devices, repair orders, diagnosis, estimates, repair work, settlement and audit. It is a JSON HTTP API for staff; there is no web frontend yet.

| Read | For |
|---|---|
| [User guide](docs/user_guide.md) | Staff tasks, order lifecycle, pricing, errors |
| [`spec/srs.md`](spec/srs.md), [`spec/openapi.yaml`](spec/openapi.yaml) | Requirements and API contract |
| [Architecture](docs/architecture.md), [stack](docs/stack.md) | How it is built, coding convention |
| [Recovery](docs/recovery.md) | Backup and restore |
| [Workflow](docs/workflow.md), [branching](docs/branching_strategy.md) | How we work |

## Requirements

- Rust 1.90 or newer
- Docker with Compose (PostgreSQL 18)
- `curl` and `jq` for the examples

## Run

```bash
docker compose up -d --wait
FIXOPS_BOOTSTRAP_ADMIN_USERNAME=admin FIXOPS_BOOTSTRAP_ADMIN_PASSWORD='change-me-now' cargo run
```

Migrations run at startup. The bootstrap variables create the first Administrator only while no staff account exists; later starts can omit them. The API listens on `http://127.0.0.1:8080/api/v1`:

```bash
curl -s http://127.0.0.1:8080/health   # ok
curl -s http://127.0.0.1:8080/api/v1/sessions -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"change-me-now"}' | jq .
```

## Test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

HTTP tests create a throwaway database per test on the Compose Postgres. The Prism suite (`cargo test --test prism`) needs a mock server; see `tests/prism/mod.rs`.

## API reference

```bash
cargo doc --workspace --no-deps --open
```

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | `postgres://fixops:fixops@localhost:5432/fixops` (from `.cargo/config.toml`) | PostgreSQL connection |
| `FIXOPS_LISTEN_ADDR` | `127.0.0.1:8080` | HTTP listen address |
| `FIXOPS_DB_MAX_CONNECTIONS` | `10` | Pool size |
| `FIXOPS_BOOTSTRAP_ADMIN_USERNAME` | unset | Username of the first Administrator, created at startup only while no staff account exists |
| `FIXOPS_BOOTSTRAP_ADMIN_PASSWORD` | unset | Its password, at least 8 characters. Set both variables or neither |
| `FIXOPS_LOG_DIR` | `var/log` | Daily JSON operational log (`fixops.log.YYYY-MM-DD`) |
| `RUST_LOG` | `info` | Log filter |
