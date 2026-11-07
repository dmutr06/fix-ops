# P5 test gate

Decision: GO
Requirements: REQ-F-012–016; BDD-005–008, 010, 012–015, 018–020.
Red: `DATABASE_URL=postgres://fixops:fixops@localhost:5432/fixops cargo test --test settlement` with local PostgreSQL access: 0 passed, 6 failed because settlement routes returned 404. Initial sandbox-only run could not connect; rerun with database access confirmed behavioral failures.
Tests also cover all 36 lifecycle pairs, rounding boundaries, terminal immutability, timestamp precision, stale order/client saves, audit rollback and lossless cancellation source. New domain/store API tests require the P5 methods and internal cancellation source field.

Migration regression: `cargo test -p fixops-infra --test cancellation_migration` fails on the old schema when cancelled rows lack the new source column; legacy status audit recovers known stages while missing history remains unknown.
Green: `cargo test -p fixops-infra --test cancellation_migration` passes; known source is backfilled and legacy rows without status history remain NULL. Full workspace verification follows integration.
