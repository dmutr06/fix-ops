FixOps: internal service for a repair workshop. Rust, axum, PostgreSQL.

Read before changing code: `docs/architecture.md` (layers, request flow, audit contract, testing) and `docs/stack.md`. Requirements are `spec/srs.md`; the API contract is `spec/openapi.yaml`. If code, docs and spec disagree, stop and ask. Never change the spec to fit the code.

## Rules

- Test first: write failing tests from the SRS acceptance criteria and BDD scenarios, commit them (`test: ...`), then implement (`feat: ...`).
- Keep the layers: no SQL outside `fixops-infra`, no HTTP types outside the root crate, no I/O in `fixops-domain`.
- Every change writes its audit entries in the same transaction. A denied action changes nothing.
- Money is whole cents (`Cents`), never floats.
- Add dependencies only with the feature that needs them, through `[workspace.dependencies]`.

## Commands

- `docker compose up -d --wait`
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`

Run all three before finishing.

## Git

Follow `docs/branching_strategy.md`. Commit messages are one line, with no body and no trailers (no `Co-Authored-By`). PR bodies are a short overview, then related tickets; no generated-by footers.
