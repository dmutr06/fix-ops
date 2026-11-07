# Administrator guard verification

- Formatting: `cargo fmt --all` passed.
- Clippy: `cargo clippy --workspace --all-targets -- -D warnings` passed.
- Tests: `cargo test --workspace` passed (214 tests, including PostgreSQL integration tests).
- Concurrency: cross-demotion regression reproduced PostgreSQL `40P01` before the lock fix and passed after using `FOR NO KEY UPDATE`.
- Review: the identified lock-order deadlock was fixed and covered by the regression test.
- CI: pending pull request checks.
