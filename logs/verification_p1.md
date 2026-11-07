# VERIFICATION P1: Staff access and accounts

**Decision:** recommendation PASS (run 2; run 1 was FAIL, its three defects are fixed and re-verified) · **Ticket:** [FIX-29](https://linear.app/fixops/issue/FIX-29) · **Scope:** REQ-F-001, REQ-F-002, REQ-F-019, REQ-NF-002, REQ-NF-003 (staff and audit part), BDD-011 and BDD-017 as far as P1 endpoints exist, [ADD-002](../spec/addenda/ADD-002-api-contract.md) §1–2 and §5 for `/sessions`, `/staff`, `/staff/{staffId}`, `/audit-logs` · **Baseline:** [SRS 1.0.2](../spec/srs.md), [ADD-001](../spec/addenda/ADD-001-permissions.md), [`openapi.yaml`](../spec/openapi.yaml), [architecture](../docs/architecture.md) · **Code:** `origin/main` at `docs: describe session, hashing and bootstrap decisions`

Run 1 below is kept as history; the current result is [Run 2](#run-2).

Checked by a performer who did not write the code. Expected values come from the SRS, the addenda, the contract and the architecture, not from code output. No product code was changed.

## Run 1

### Commands

| Command | Result |
|---|---|
| `cargo fmt --all --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo test --workspace` | 60 passed, 0 failed: `staff_access` 28, `permissions` 3, `contract` 3, `platform` 2, `fixops-domain` 16, `fixops-app` 6, `fixops` lib 2 |
| `cargo run` on a fresh database, bootstrap `root.admin`, `FIXOPS_LISTEN_ADDR=127.0.0.1:18080` | Migrations ran, first Administrator created, 116 requests served, no `ERROR`/`WARN` lines, no `500` |
| HTTP checks (scripted, standard-library HTTP client) | 99 checks: 89 as expected, 6 mismatches (D-1 ×1, D-2 ×4, D-3 ×1), 4 notes |
| `psql` via `docker exec` on the same database | See [Database checks](#database-checks) |
| Server stopped, database dropped | Done |

### Requirements

| Requirement | Check | Evidence | Result |
|---|---|---|---|
| REQ-F-001 / AC-F-001 | Every protected P1 route needs a valid bearer session | No token, unknown token, `Basic` scheme, expired session (test), disabled account's token → `401 UNAUTHENTICATED` on `GET`/`POST /staff`, `PATCH /staff/{id}`, `GET /audit-logs`; body only `code`, `message` | PASS |
| REQ-F-001 | Signed-in user can use permitted actions | Each role signs in (`201 SessionResponse`) and reads `/audit-logs` (`200`) | PASS |
| REQ-F-001 | Sign-in does not reveal which accounts exist (architecture § Sessions) | Same `401` body for wrong password and unknown user, but median latency 233 ms vs 1 ms | FAIL (D-1) |
| BDD-017 | Visitor without sign-in gets no record contents | Holds for P1 routes. Client and order routes do not exist yet (`/clients` → `404`); recheck in P2–P4 | PASS (P1 part) |
| REQ-F-002 / AC-F-002 | Role matrix equals `x-roles` for all 25 operations | `role_matrix_matches_x_roles` and `every_protected_operation_has_an_action` parse `openapi.yaml`; `Role::can` is the only check | PASS |
| REQ-F-002, ADD-002 §2 | Receptionist and Technician get `403` on `/staff`, nothing changes | `403 FORBIDDEN` on list, create, role change, disable, disable of an Administrator, unknown id; staff list and audit log unchanged | PASS |
| REQ-F-002, ADD-002 §2 | `403` for other roles regardless of request body | Receptionist/Technician `POST /staff` with a 7-character password and `PATCH` with `role: "Manager"` → `400 VALIDATION_ERROR` | FAIL (D-2) |
| BDD-011 | Technician denied, record unchanged | Tier endpoint is P2. P1 analogue (Technician changes a staff role) → `403`, record and audit unchanged | PASS (P1 part) |
| ADD-001 §2, BDD-020 | Administrator gets staff actions through the matrix, no bypass | `administrator_inherits_every_staff_action`; second Administrator creates staff | PASS |
| REQ-F-019 / AC-F-019 | Administrator creates accounts in every role | `201 StaffAccount`, `enabled: true`; each new account signs in with its role | PASS |
| REQ-F-019 | Update role and enabled flag | `200 StaffAccount`; no-op update writes no audit row | PASS |
| REQ-F-019 | Disabled account cannot sign in | Correct password → `403 FORBIDDEN` with `details.username`; wrong password → `401`; existing token → `401` at once; other sessions unaffected; re-enabling does not revive the old token | PASS |
| REQ-F-019 | Assigned role controls actions, change applies to existing session | Technician token: `403` → promoted → `200` → demoted → `403` | PASS |
| REQ-F-019, contract | Validation and conflicts | 7 vs 8 characters, blank username, unknown role, missing/unknown/null fields, malformed JSON, wrong Content-Type → `400`; duplicate (also after trim) → `409 CONFLICT`; unknown id → `404`; ids `0`, `-1`, `abc`, `1.5`, overflow → `400` | PASS |
| REQ-F-019, contract | `StaffUpdateRequest` values are non-nullable | `{"role": null, "enabled": true}` → `200` | FAIL (D-3) |
| REQ-NF-002 / AC-NF-002 | Individual accounts; every change names one actor | Every audit row has `actor_id`; bootstrap row is attributed to the bootstrap account itself | PASS |
| REQ-NF-003 / AC-NF-003 | Actor, time, record, field, old and new value; available after reopen | Creation: 4 rows (`username`, `role`, `enabled`, `password` with both values `NULL`); updates: one row per changed field with previous and new value; filters by `record_type`/`record_id`; every role reads them | PASS |
| REQ-NF-003 | Change and audit in one transaction; denied action writes nothing | `insert_audit` runs on the record's transaction; denied and invalid requests leave the audit log unchanged | PASS |
| REQ-NF-003 | Audit is append-only | `UPDATE` and `DELETE` raise `audit_log is append-only`; see limitation on `TRUNCATE` | PASS |
| ADD-002 §5 | Unified `ErrorResponse` | Every error body has only `code`, `message`, optional `details`; codes from the contract enum | PASS |

### Manual HTTP checks

| Request | Expected | Actual |
|---|---|---|
| `POST /sessions` bootstrap admin | `201`, `token` + `StaffAccount` | `201`, 43-character token, keys `{token, staff}` |
| `POST /staff` as Administrator, each role | `201 StaffAccount` | `201`, keys `{id, username, role, enabled}` |
| `POST /sessions` each new account | `201` with its role | `201` |
| `GET /staff` as Administrator | `200` array of `StaffAccount` | `200`, no password or hash fields |
| `GET`/`POST /staff`, `PATCH /staff/{id}`, `GET /audit-logs` without token | `401 UNAUTHENTICATED` | `401`, `{code, message}` only |
| `GET /staff` unknown token; `Authorization: Basic <valid token>` | `401` | `401` |
| Receptionist, Technician: list, create, change role, disable, disable Administrator, unknown id | `403 FORBIDDEN` | `403`; staff list unchanged afterwards |
| Receptionist, Technician: `POST /staff` with invalid body; `PATCH` with unknown role | `403 FORBIDDEN` | `400 VALIDATION_ERROR` (D-2) |
| Receptionist, Technician: `GET /audit-logs` | `200` | `200` |
| `POST /staff` 7-char password / blank username / unknown role / missing role / extra `enabled` / null or numeric username | `400 VALIDATION_ERROR` | `400`, `details.field` where the domain rejects |
| `POST /staff` 8-char password | `201` | `201` |
| `POST /staff` malformed JSON / `text/plain` / JSON array | `400` | `400` |
| `POST /staff` duplicate username, also with surrounding spaces | `409 CONFLICT`, `details.username` | `409` |
| `PATCH /staff/{id}` `{}` / unknown role / `enabled: "no"` / `username` / `password` / `{"enabled": null}` | `400` | `400` |
| `PATCH /staff/{id}` `{"role": null, "enabled": true}` | `400` | `200` (D-3) |
| `PATCH /staff/0`, `-1`, `abc`, `1.5`, `99999999999999999999` | `400`, `details.field = staffId` | `400` |
| `PATCH /staff/999999` | `404 NOT_FOUND` | `404` |
| `POST /sessions` wrong password; unknown user | identical `401` | identical `401` bodies |
| Same, latency (median of 8, debug build) | Not distinguishable | 233 ms vs 1 ms (D-1) |
| `POST /sessions` missing field / empty username / empty password / extra field / malformed JSON | `400` | `400` |
| Disable Technician, reuse its token | `401` | `401` |
| Disabled Technician signs in, correct / wrong password | `403` / `401` | `403` with `details.username` / `401` |
| Receptionist token after Technician disabled | `200` | `200` |
| Re-enable, reuse old token; sign in again | `401`; `201` | `401`; `201` |
| Promote Technician to Administrator, same token `GET /staff`; demote to Receptionist, same token | `200`; `403` | `200`; `403` |
| `GET /audit-logs` | `200` array of `AuditEntry` | `200`, exactly the 8 contract keys, RFC 3339 `recorded_at` |
| `GET /audit-logs?record_type=StaffAccount&record_id={tech}` | Only that record, oldest first | 8 rows: 4 creation, then `enabled` ×2, `role` ×2 with old/new values |
| `GET /audit-logs?record_type=Invoice`, `record_type=`, `record_id=0`, `-2`, `x`, `foo=1` | `400` | `400` |
| `GET /audit-logs?record_type=Client` | `200 []` | `200 []` |
| Unknown route `/api/v1/nowhere` | `404 NOT_FOUND` | `404` |
| Note: `POST /staff` `Max.Tech` while `max.tech` exists | Not specified | `201`; usernames are case-sensitive |
| Note: Administrator disables own account | Not specified | `200`; its token then gets `401` |
| Note: `DELETE /staff` | Not in contract | `405` with empty body, not `ErrorResponse` |
| Note: `GET /clients` without token | P2 route | `404`; BDD-017 for clients and orders is verified in later packages |

### Database checks

| Check | Expected | Actual | Result |
|---|---|---|---|
| `staff_accounts.password_hash` | argon2 PHC string, no plaintext | All 7 rows `$argon2id$v=19$m=19456,t=2,p=1$…`, 97 characters | PASS |
| `sessions.token_hash` | 32-byte SHA-256, token never stored | `bytea`, length 32 on every row (also a `CHECK`); SHA-256 of the issued token matches 1 row; token text found in 0 rows | PASS |
| Sessions of a disabled account | Deleted | No rows for the disabled Administrator; Technician's old row gone after disable | PASS |
| Session expiry | 12 hours | `expires_at - now() > 11 h` on every row | PASS |
| `audit_log` completeness | Actor, time, record type, record id, field never null | 33 rows, 0 incomplete; `password` rows have both values `NULL` | PASS |
| `UPDATE audit_log …` / `DELETE FROM audit_log …` | Refused | `ERROR: audit_log is append-only`, row unchanged | PASS |

### Test review (risk R-11)

The tests assert contract values (status, `ErrorResponse.code`, exact key sets of `StaffAccount` and `AuditEntry`), and the role matrix is compared with `x-roles` parsed from `openapi.yaml`, so code and test cannot drift together there. Denied actions read the staff list and audit log back. Gaps that let the defects through:

| Gap | Effect |
|---|---|
| Denied-role tests only send valid bodies | D-2 not caught |
| No test for `null` in `StaffUpdateRequest` fields | D-3 not caught |
| Indistinguishability test compares bodies only, not timing | D-1 not caught |
| `401` tests cover only `GET /staff` and `GET /audit-logs`, not `POST /staff` or `PATCH /staff/{id}` | Verified manually above |
| Password storage test asserts only `hash != plaintext`; no test of argon2 format, token hash length, or audit `UPDATE`/`DELETE` refusal | Verified manually above |

### Defects

| ID | Severity | Requirement | Description |
|---|---|---|---|
| D-1 | Medium | REQ-F-001; architecture § Sessions | Sign-in reveals which usernames exist through response time. `App::sign_in` (`crates/app/src/staff.rs`) returns `invalid_credentials()` as soon as `find_credentials` finds no account, without running argon2; a wrong password for a real account runs argon2. Measured median 233 ms vs 1 ms (debug build; a release build narrows but does not remove the gap). Expected: an unknown username costs the same password check, for example by verifying against a fixed dummy hash. |
| D-2 | Low | REQ-F-002, ADD-002 §2; architecture § Request flow ("403 before anything else") | `POST /staff` and `PATCH /staff/{staffId}` validate the body before checking the role. `src/http/staff.rs` parses `Username`, `Password` and `role`, and the `Json` extractor rejects malformed bodies, before `create_staff`/`update_staff` call `actor.require`. A Receptionist or Technician with an invalid body gets `400 VALIDATION_ERROR` instead of `403 FORBIDDEN`, and learns the validation rules. Nothing is written. |
| D-3 | Low | REQ-F-019, contract `StaffUpdateRequest` | `role` and `enabled` are non-nullable in the contract, but `StaffUpdateRequest` uses `Option<…>` without rejecting explicit `null`, so `{"role": null, "enabled": true}` is accepted (`200`) and `null` is treated as absent. Expected `400 VALIDATION_ERROR`. |

### Limitations (run 1)

- Performance (REQ-NF-001) and backups (REQ-NF-004) are outside P1.
- BDD-011 (loyalty tier) and BDD-017 for clients, devices and orders need P2–P4 routes; only the P1 equivalents were checked. The role matrix for those operations is checked statically against `x-roles`.
- Audit append-only is a row-level trigger: `TRUNCATE audit_log` is not blocked (checked inside a rolled-back transaction), and the service connects as the database superuser, which could disable the trigger. A restricted database role for the service would close this; it is a deployment decision, not a P1 acceptance criterion.
- Not specified, for the gate owner: an Administrator can disable its own account or the last enabled Administrator (possible lockout until the bootstrap variables are used on an empty database); usernames are case-sensitive; `405` responses carry no `ErrorResponse` body.
- Timing was measured on a local debug build with 8 samples per case.

## Run 2

**Result:** PASS · **Code:** `fix-29/staff-access-fixes` = run 1 log, then `test: cover sign-in timing, role-before-body and null fields`, `fix: hide unknown usernames, check roles first, reject null fields`, `docs: describe check order, null fields and audit log triggers`, on top of `main` with operational logging · **Gate-owner decisions:** fix D-1, D-2, D-3 and block `TRUNCATE` on `audit_log`; accept the limitations listed under [Limitations](#limitations)

### Commands

| Command | Result |
|---|---|
| `cargo fmt --all --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo test --workspace` | 74 passed, 0 failed: `staff_access` 32 (+4), `logging` 4, `permissions` 3, `contract` 3, `platform` 2, `fixops-domain` 16, `fixops-app` 8 (+2), `fixops-infra` 3 (+3), `fixops` lib 3 (+1) |
| `cargo run --release` on a fresh database, `FIXOPS_LOG_DIR` in a temp dir | Migrations ran (including `audit_log_no_truncate`), first Administrator created, 202 requests served, no `ERROR` lines, no `500` |
| HTTP checks (run 1 script plus 22 new checks) | 121 checks: 117 as expected, 0 mismatches, 4 notes (same as run 1) |
| Server stopped, database dropped | Done |

### Review of the test and fix commits

| Item | Finding | Result |
|---|---|---|
| `denied_roles_get_403_whatever_the_body` | Receptionist and Technician get `403` for invalid, incomplete, unknown-field and malformed bodies, `null` fields, and ids `0`, `abc`, `999999`; staff list and audit log read back unchanged. Expresses ADD-002 §2 and §5 | OK |
| `missing_session_gets_401_whatever_the_body` | No token with malformed body or invalid id → `401`; nothing changes. `401` comes before `403` and `400` | OK |
| `null_fields_in_staff_update_are_rejected` | `null` in `role` or `enabled`, alone or with a valid field → `400`, nothing written. Matches non-nullable `StaffUpdateRequest` | OK |
| `audit_log_cannot_be_changed_deleted_or_truncated` | `UPDATE`, `DELETE`, `TRUNCATE` refused with the trigger message; rows unchanged | OK |
| `unknown_username_costs_the_same_password_check` (app unit) | Counts password checks: unknown user and wrong password both do exactly one, same error kind. A behavioural proxy for timing that does not depend on the machine | OK |
| `password.rs` unit tests | Dummy hash never verifies; it uses the same argon2 parameters as real hashes | OK |
| Fix D-1 | `verify_password` takes `Option<&PasswordHash>`; with no account it checks a lazily built dummy hash of the same parameters and answers `false` | OK |
| Fix D-2 | `/staff` handlers take the body and path id as `Result<…>`, call `app.authorize` first, then unwrap. Extractor order (`SignedIn` first) gives `401` → `403` → `400` → `404`/`409`. The use cases still check the role (defence in depth) | OK |
| Fix D-3 | `json::not_null` on `StaffUpdateRequest` fields; absence stays allowed, explicit `null` → `400` | OK |
| Generalization | The pattern is documented in the architecture for every future handler. `GET /audit-logs` still parses its query before `authorize`; all roles may read it, so responses cannot differ, but a `400` there is logged without `action` (O-1) | OK, note |

### Former mismatches

| Request | Run 1 | Run 2 |
|---|---|---|
| Receptionist / Technician `POST /staff` with 7-character password | `400` | `403 FORBIDDEN` |
| Receptionist / Technician `PATCH /staff/{id}` `role: "Manager"` | `400` | `403 FORBIDDEN` |
| Receptionist / Technician `POST /staff` malformed JSON, missing fields, unknown field (new) | not run | `403 FORBIDDEN` |
| Receptionist / Technician `PATCH` `{}`, `role: null`, `enabled: "no"`, ids `0`, `abc` (new) | not run | `403 FORBIDDEN` |
| No token, `POST /staff` malformed JSON; `PATCH /staff/0` (new) | not run | `401 UNAUTHENTICATED` |
| Administrator `PATCH` `{"role": null, "enabled": true}` | `200` | `400 VALIDATION_ERROR` |
| Administrator `PATCH` `{"role": "Technician", "enabled": null}`, `{"role": null}` (new) | not run | `400 VALIDATION_ERROR` |
| Sign-in latency, median of 20, release build: wrong password vs unknown user | 233 ms vs 1 ms (debug, 8 samples) | 11.9 ms vs 11.7 ms; repeat with other names 11.8 ms vs 11.7 ms |
| `TRUNCATE audit_log` | Allowed | `ERROR: audit_log is append-only`, 33 rows kept |

All other run 1 checks gave the same results as in run 1 (sign-in, role per account, disable and token reuse, role change and token reuse, validation, `404`, `409`, audit content and filters).

### Database checks

| Check | Actual | Result |
|---|---|---|
| Password hashes | 7 of 7 `$argon2id$v=19$m=19456,t=2,p=1$` | PASS |
| Session tokens | 3 of 3 rows 32-byte `bytea` | PASS |
| Audit rows | 33, password rows have both values `NULL` | PASS |
| Triggers | `audit_log_append_only` (row, `UPDATE`/`DELETE`), `audit_log_no_truncate` (statement, `TRUNCATE`) | PASS |
| `UPDATE`, `DELETE`, `TRUNCATE` on `audit_log` | All refused; count still 33 | PASS |

### Operational log

JSON-lines file `fixops.log.YYYY-MM-DD` in `FIXOPS_LOG_DIR`, 421 lines.

| Check | Expected | Actual | Result |
|---|---|---|---|
| Every request logged once | One `request finished` per request | 202 lines, 202 distinct `request_id`s, none missing | PASS |
| Level by status | `INFO` < 400, `WARN` 4xx, `ERROR` 5xx | 0 mismatches (30 `INFO`, 172 `WARN`, no 5xx) | PASS |
| Request id | Echoed from `x-request-id`, generated when missing | `verify-rid-0001` echoed; UUID generated for `/health` | PASS |
| Actor | `actor_id`, `role` on authenticated requests | Present on all 94 authenticated API requests not answered `401`, except 3 unrouted requests (`/clients`, `/nowhere`, `DELETE /staff`) that have no session extractor | PASS |
| Action | `action` where known | Present on every routed authenticated request, except 6 `GET /audit-logs` answered `400` (O-1) | PASS, note |
| Denials | `action denied` with actor, role, action | 34 lines = 35 `403` responses minus 1 disabled-account sign-in (not a role denial) | PASS |
| Rejections | `request rejected` with `code` for each 4xx | 171 lines = 172 4xx minus the `405` (axum default, not an `ApiError`) | PASS |
| Changes | `record changed` with field names only | 12 lines, e.g. `fields: "enabled"`, no values | PASS |
| Secrets | No password, hash or token | 0 hits for the bootstrap password, staff password, wrong password, `$argon2`, the issued admin token, `Bearer`, `Authorization`; `password` appears only as a field name or in validation messages | PASS |

### Defects

None. D-1, D-2 and D-3 from run 1 are fixed and re-verified.

| ID | Kind | Note |
|---|---|---|
| O-1 | Observation | `GET /audit-logs` parses its query before `authorize`, so an invalid filter is logged without `action`. Responses are correct because every signed-in role may read the audit log. Worth aligning with the documented handler pattern when P2 adds more query parameters. |
| O-2 | Observation | `405` responses have an empty body and no `request rejected` line. The contract defines no `405`. |

## Limitations

- Performance (REQ-NF-001) and backups (REQ-NF-004) are outside P1.
- BDD-011 (loyalty tier) and BDD-017 for clients, devices and orders need P2–P4 routes; only the P1 equivalents were checked here. The role matrix for those operations is checked statically against `x-roles`.
- Accepted by the gate owner: the service connects to the database as a superuser, which could disable the audit triggers; moved to P7 operations.
- Accepted by the gate owner: an Administrator can disable its own account or the last enabled Administrator; the spec is silent; candidate addendum.
- Accepted by the gate owner: usernames are case-sensitive (`Max.Tech` and `max.tech` can coexist); the spec is silent; candidate addendum.
- Timing was measured on one local machine, release build, 20 sequential samples per case; it shows the gap is closed, not a statistical bound.
