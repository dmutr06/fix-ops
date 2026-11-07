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

## P3 combined release run (2026-11-07)

Branch based on P3 `7b82407`; release binary built from `40efe03`. macOS Darwin arm64, Rust 1.99.1, Python 3.15.2, PostgreSQL 18 in the existing Compose service. A fresh `fixops_performance_p3` database started with 10,000 orders; 10 concurrently signed-in staff executed 20 cycles each. The full Rust test suite was running concurrently on the same machine and database server.

| Action | Samples | p50 | p95 | Maximum | Errors |
| --- | ---: | ---: | ---: | ---: | ---: |
| Client search | 200 | 26.5 ms | 53.9 ms | 66.5 ms | 0 |
| Device search | 200 | 19.5 ms | 39.7 ms | 66.8 ms | 0 |
| Order open | 200 | 12.2 ms | 40.2 ms | 56.5 ms | 0 |
| Order save (create) | 200 | 37.5 ms | 103.9 ms | 130.7 ms | 0 |
| Allowed transition | 200 | 25.5 ms | 54.3 ms | 100.6 ms | 0 |

`python3 scripts/check-performance.py` exited 0. Every category's HTTP p95 was below two seconds. The service's JSON logs recorded 801 HTTP 200 and 219 HTTP 201 responses, including setup; no rejections, denials, or failures, and 400 RepairOrder change events. A database count after the run confirmed 10,200 orders and 200 Diagnosing orders. The local API check passes; AC-NF-001's request-to-visible-result timing requires browser validation.

`docker compose -p fix-ops up -d --wait`, `cargo build --release`, `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all passed; the full suite ran 174 tests with no failures. Tests had access to the local PostgreSQL server.
