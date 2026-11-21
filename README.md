# FixOps

Internal service for a PC and electronics repair workshop: clients, devices, repair orders, diagnosis, estimates, repair work, settlement and audit. Staff use it in the browser; the same JSON API serves scripts.

| Read | For |
|---|---|
| [User guide](docs/user_guide.md) | Staff tasks, order lifecycle, pricing, errors |
| [`spec/srs.md`](spec/srs.md), [`spec/openapi.yaml`](spec/openapi.yaml) | Requirements and API contract |
| [Architecture](docs/architecture.md), [stack](docs/stack.md) | How it is built, coding convention |
| [Recovery](docs/recovery.md) | Backup and restore |
| [Workflow](docs/workflow.md), [branching](docs/branching_strategy.md) | How we work |

## Requirements

- Rust 1.90 or newer
- Node.js 24 and pnpm 12 for the web app
- Docker with Compose (PostgreSQL 18)
- `curl` and `jq` for the examples

## Run

```bash
docker compose up -d --wait
(cd web && pnpm install && pnpm build)
FIXOPS_BOOTSTRAP_ADMIN_USERNAME=admin FIXOPS_BOOTSTRAP_ADMIN_PASSWORD='change-me-now' cargo run
```

Open `http://127.0.0.1:8080` and sign in as `admin`. For demo data on an empty database, run `FIXOPS_ADMIN_USERNAME=admin FIXOPS_ADMIN_PASSWORD='change-me-now' scripts/seed.sh`: three staff (password `seed-pass-123`), four clients, eight computers and orders at every stage. Migrations run at startup. The bootstrap variables create the first Administrator only while no staff account exists; later starts can omit them. Debug builds read `web/dist` from disk; release builds embed it, so build the web app first. For live reload, run `pnpm dev` in `web/` next to `cargo run`.

The API listens on `http://127.0.0.1:8080/api/v1`:

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

HTTP tests create a throwaway database per test on the Compose Postgres. The web app:

```bash
cd web
pnpm lint && pnpm typecheck
pnpm exec playwright install chromium   # once
pnpm e2e   # builds the app and runs the service on a fresh fixops_e2e database
```

The Prism suite (`cargo test --test prism`) needs a mock server; see `tests/prism/mod.rs`.

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
