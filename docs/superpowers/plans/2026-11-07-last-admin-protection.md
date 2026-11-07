# Keep one enabled Administrator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent staff-account updates from removing the last enabled Administrator while retaining a safe promotion path if a database has none.

**Architecture:** Add the rule to the SRS and the existing staff PATCH contract. In `fixops-infra`, explicitly set the update transaction to `READ COMMITTED`, lock the target row, and use a transaction-scoped advisory lock before counting other enabled Administrators. Keep the update, session revocation, and audit insert in the same transaction; map the guard through `StoreError` to the existing `409 CONFLICT` response.

**Tech Stack:** Rust workspace, axum, Tokio, SQLx, PostgreSQL 18, Docker Compose. No new dependencies.

**Spec:** [Design](../specs/2026-11-07-last-enabled-administrator-design.md); new [ADD-003](../../../spec/addenda/ADD-003-last-enabled-administrator.md).

## Global Constraints

- Test first: write failing tests from SRS acceptance criteria and BDD, commit them (`test:`), then implement (`feat:`).
- SQL stays in `fixops-infra`; HTTP DTOs stay in the root crate; domain stays free of I/O.
- A denied update changes neither the staff row, sessions, nor audit rows.
- A successful update and its audit insert remain in one transaction.
- Dependencies are added only when required through `[workspace.dependencies]`; this change adds none.
- Commit messages are one line, without bodies or trailers.

## Review Focus

- Two demotions in parallel cannot both leave zero enabled Administrators; pin this in `staff_access::concurrent_last_administrator_demotions_leave_one_enabled`.
- A failed self-demotion preserves the caller's active session and audit history; pin this in `staff_access::last_enabled_administrator_cannot_demote_or_disable`.
- Disabled Administrators do not count toward the invariant; pin this in `staff_access::disabled_administrator_does_not_satisfy_last_admin_guard`.
- A database already at zero enabled Administrators can recover by promoting an account; pin this in `staff_access::promotion_restores_administrator_access`.
- A valid demotion succeeds when a different enabled Administrator remains and still revokes the updated account's sessions; pin this in `staff_access::another_enabled_administrator_allows_demotion`.

---

### Task 1: Approve the SRS and API contract change

**Files:**
- Create: `spec/addenda/ADD-003-last-enabled-administrator.md`
- Modify: `spec/srs.md`
- Modify: `spec/openapi.yaml`
- Modify: `spec/traceability.md`
- Create: `logs/spec_addendum_gate.md`

**Interfaces:**
- Produces requirement `REQ-F-020`, acceptance criterion `AC-F-020`, scenario `BDD-021`, and a `409 CONFLICT` example on `PATCH /api/v1/staff/{staffId}`.

- [ ] Add ADD-003: reject only an update that would demote or disable the last *currently enabled* Administrator. Other enabled Administrators count; disabled accounts do not. A promotion to Administrator remains allowed even if there are currently none.
- [ ] Bump the SRS to 1.0.3, add REQ-F-020 / AC-F-020 / BDD-021, and link ADD-003 in traceability.
- [ ] Add a staff PATCH `409 CONFLICT` response example for the last-enabled-Administrator guard; keep the error model and endpoint unchanged.
- [ ] Record the delegated spec gate decision and test scope in a short log.
- [ ] Commit spec and gate as `spec: protect the last enabled administrator`.

### Task 2: Add failing staff-management tests

**Files:**
- Modify: `tests/staff_access.rs`

**Interfaces:**
- Consumes: the existing staff PATCH route, `TestApp`, account fixtures, audit query helpers, and `App::update_staff`.
- Produces: tests named in Review Focus and assertions for status, account, session, audit, and concurrent outcomes.

- [ ] Write HTTP tests for blocking self-demotion and self-disable when there is no other enabled Administrator. Assert `409 CONFLICT`, unchanged account, valid old session, and unchanged audit count.
- [ ] Test a successful demotion when another enabled Administrator remains; assert the former account's sessions are revoked and its changes are audited.
- [ ] Disable the other Administrator first, then assert disabling the remaining enabled Administrator is rejected.
- [ ] Create a zero-enabled-Administrator fixture through direct setup SQL and call the app use case with an authorized test actor to prove that promoting a Receptionist is allowed.
- [ ] Race demotions of two enabled Administrators. Assert exactly one succeeds, one returns conflict, and the database retains one enabled Administrator with only one set of audit rows.
- [ ] Run `cargo test --test staff_access` and confirm the new guard tests fail against current behavior.
- [ ] Commit tests only as `test: protect the last enabled administrator`.

### Task 3: Implement the atomic guard

**Files:**
- Modify: `crates/app/src/error.rs`
- Modify: `crates/infra/src/staff.rs`
- Modify: `tests/staff_access.rs`

**Interfaces:**
- Produces: `StoreError::LastEnabledAdministrator`, mapped to `AppError::conflict`.
- The `StaffStore::update_staff` signature remains unchanged.

- [ ] Add a store error for the rejected last-Administrator transition and map it to the existing conflict error.
- [ ] At the start of `PgStore::update_staff`, issue `SET TRANSACTION ISOLATION LEVEL READ COMMITTED` before any query.
- [ ] Lock the target `staff_accounts` row with `FOR UPDATE`. If its current role is Administrator and enabled is true, and the requested result is not Administrator+enabled, acquire one fixed transaction advisory lock, count other enabled Administrators, and return `LastEnabledAdministrator` when the count is zero.
- [ ] Keep the role/enabled update, optional session deletion, and audit insertion in that same transaction. Do not guard promotions or changes that leave an enabled Administrator.
- [ ] Run the named staff-access tests and confirm they pass, including the concurrent case.
- [ ] Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` against Docker PostgreSQL.
- [ ] Commit implementation as `feat: protect the last enabled administrator`.

### Task 4: Verify and close the package

**Files:**
- Create: `logs/verification_add003.md`

- [ ] Record passing formatting, Clippy, workspace test, and review results in a short verification log.
- [ ] Commit as `logs: record administrator guard verification`.
- [ ] Rebase onto latest `origin/main`, open a concise PR with related ticket(s) at the end, and merge after CI passes.
- [ ] Update the linked Linear issue and check off verified acceptance criteria.
