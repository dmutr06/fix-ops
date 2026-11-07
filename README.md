# FixOps

Internal service for a PC and electronics repair workshop: clients, devices, repair orders, diagnosis, estimates, repair work, settlement and audit.

- Requirements: [`spec/srs.md`](spec/srs.md), API contract: [`spec/openapi.yaml`](spec/openapi.yaml)
- How it's built: [`docs/architecture.md`](docs/architecture.md), [`docs/stack.md`](docs/stack.md)
- How we work: [`docs/workflow.md`](docs/workflow.md), [`docs/branching_strategy.md`](docs/branching_strategy.md)

## Run

```bash
docker compose up -d --wait
cargo run
```

The API listens on `http://127.0.0.1:8080/api/v1`. Migrations run at startup.

## Test

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

HTTP tests create a throwaway database per test on the Compose Postgres.

## Configuration

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | `postgres://fixops:fixops@localhost:5432/fixops` (from `.cargo/config.toml`) | PostgreSQL connection |
| `FIXOPS_LISTEN_ADDR` | `127.0.0.1:8080` | HTTP listen address |
| `FIXOPS_DB_MAX_CONNECTIONS` | `10` | Pool size |
| `RUST_LOG` | `info,tower_http=debug` | Log filter |
