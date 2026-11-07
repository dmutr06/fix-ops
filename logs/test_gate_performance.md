# Performance verification

## Baseline on main (2026-11-07)

Release build at `6f503bc`, PostgreSQL 18 in the local Compose container, dedicated database with 10,000 seeded orders, 10 signed-in administrators, 10 concurrent workers, 200 samples per category. HTTP response times, not visible browser timings.

| Action | p50 | p95 | Maximum | Errors |
| --- | ---: | ---: | ---: | ---: |
| Client search | 15.1 ms | 24.2 ms | 46.8 ms | 0 |
| Device search | 8.5 ms | 13.5 ms | 20.3 ms | 0 |
| Order open | 3.5 ms | 7.8 ms | 9.8 ms | 0 |
| Order save (create) | 10.9 ms | 16.6 ms | 28.2 ms | 0 |
| Allowed transition | — | — | — | 200 (404) |

`python3 scripts/check-performance.py` exited 1: diagnosis is not implemented on this baseline. Error response latency is not transition performance. The JSON operational log contains 601 successful 200 responses, 219 successful 201 responses, and 200 404 responses; all 200 rejections are `NOT_FOUND` for the unavailable diagnosis route. This includes setup requests. A preceding exploratory run had 200 save conflicts from an incorrect assignee in the fixture; the fixture was corrected and the table above is from a fresh database.

The release build and `cargo fmt --all -- --check` passed. `cargo clippy --workspace --all-targets -- -D warnings` passed on main. An attempted main-only `cargo test --workspace` could not access PostgreSQL inside the sandbox; the 35 intake tests failed during setup with `Operation not permitted`, not application assertions. Full verification is pending on the combined branch with database access. Browser-visible timing remains pending.
