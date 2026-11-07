# P6 corrections gate

Scope: REQ-F-018 / BDD-016, ADD-001 and ADD-002; parent-supplied P6 criteria replace the absent continuation brief. No contract changes or dependencies.

Red: `cargo test --test corrections` against PostgreSQL: 5 failed, all at PATCH with 405 (missing route), including the authorization case expecting 403. Domain acceptance tests specify combined-candidate validation and safe legacy cancellation handling.

Implementation gate: validate the final candidate, enforce lifecycle transitions and terminal status retention, check links and Technician role, recalculate terminal settlement at current tier, retain cancellation source, and save with version plus field audit atomically. Unknown legacy cancellation source permits metadata corrections only; changes requiring source evidence are rejected.

Database: worktree Compose project collided with the existing port 5432; `docker compose -p fix-ops up -d --wait` succeeded using the existing healthy PostgreSQL service. Database tests require sandbox network escalation.
