# VERIFICATION P2: Client, device and order intake

**Decision:** recommendation PASS (run 2; run 1 was FAIL, its defect D-1 is fixed and re-verified) · **Ticket:** [FIX-32](https://linear.app/fixops/issue/FIX-32) · **Scope:** REQ-F-003, REQ-F-004, REQ-F-005, REQ-F-017, with REQ-F-001, REQ-F-002, REQ-NF-002, REQ-NF-003 for the new endpoints; BDD-001, BDD-002, BDD-011, BDD-015 (tier persistence), BDD-017; [ADD-002](../spec/addenda/ADD-002-api-contract.md) §1, §4, §5 for `/clients*`, `/devices*`, `GET`/`POST /orders`, `GET /orders/{orderId}` · **Baseline:** [SRS 1.0.2](../spec/srs.md), [ADD-001](../spec/addenda/ADD-001-permissions.md), [ADD-002](../spec/addenda/ADD-002-api-contract.md), [`openapi.yaml`](../spec/openapi.yaml), [glossary](../spec/glossary.md), [architecture](../docs/architecture.md) · **Code:** run 1 `fix-30-31/intake` at `docs: describe intake checks, search and settlement function`; run 2 `fix-32/intake-fixes` at `fix: reject stale client and device updates`

Run 1 below is kept as history; the current result is [Run 2](#run-2).

Checked by a performer who did not write the code. Expected values come from the SRS, the addenda, the glossary, the contract and the architecture, not from code output. No product code was changed.

## Run 1

### Commands

| Command | Result |
|---|---|
| `cargo fmt --all --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo test --workspace` | 133 passed, 0 failed: `intake` 35 (new), `staff_access` 32, `logging` 4, `permissions` 3, `contract` 3, `platform` 2, `fixops-domain` 39 (+23), `fixops-app` 8, `fixops-infra` 4 (+1), `fixops` lib 3 |
| `cargo run --release` on a fresh database, `FIXOPS_LISTEN_ADDR=127.0.0.1:18082`, `FIXOPS_LOG_DIR` in a temp dir | Migrations ran (including `intake`), first Administrator created, 662 requests sent, no `ERROR` lines, no `5xx` |
| HTTP checks (scripted, standard-library HTTP client) | 268 checks: 267 as expected, 1 mismatch (D-1); a second script of 40 concurrent rounds confirms D-1 for tiers and devices |
| `psql` via `docker exec` on the same database | See [Database checks](#database-checks) |
| Server stopped, database dropped | Done |

### Requirements

| Requirement | Check | Evidence | Result |
|---|---|---|---|
| REQ-F-003 / AC-F-003 | Client with name and phone is saved and can be reopened | `POST /clients` → `201 Client`, `loyalty_tier: "Standard"`; `GET /clients/{id}` returns the same body; email optional (absent or `null` → `email: null`) | PASS |
| REQ-F-003 / AC-F-003 | Missing either required value prevents saving | Missing, empty, blank, `null` and numeric `name`/`phone` → `400 VALIDATION_ERROR` (`details.field` for empty/blank); nothing stored, no audit row | PASS |
| REQ-F-003 | Update | `PATCH` changes only the given fields; `email: null` clears; `{}`, `null` in `name`/`phone`, unknown fields (`loyalty_tier`, `id`) → `400`; no-op update writes no audit row | PASS |
| REQ-F-003, ADD-002 §1 | Search | `q` matches name or phone, any case, substring; `%` and `_` literal; empty `q`, unknown or repeated parameters → `400` | PASS |
| REQ-F-004 / AC-F-004 | Device with type, serial number and symptoms persists and can be retrieved | `POST /devices` → `201 Device`; reopened equal; values trimmed | PASS |
| REQ-F-004 / AC-F-004, BDD-002 | Missing any value prevents saving | Missing, `null`, empty, blank `serial_number`/`reported_symptoms`; unknown `device_type` (`laptop`, `Tablet`, `""`) → `400` with `details.field`; unknown client → `404`, `details.field = client_id`; no record, no audit | PASS |
| REQ-F-004 | Update and list | `PATCH` keeps other fields; `client_id` in the body → `400` (not in `DeviceUpdateRequest`); `GET /devices?client_id=`, `?q=`, both combined filter as specified | PASS |
| REQ-F-005 / AC-F-005, BDD-001 | Order saves all links and fields in `Received` | `201 RepairOrder` with all 16 contract keys: `status: "Received"`, `diagnostic_findings`, `estimate`, `client_decision`, `payment`, `pickup` `null`, all `*_cents` integer `0`, `repair_checked: false`; reopened equal; Receptionist and Administrator | PASS |
| REQ-F-005 / AC-F-005, BDD-002 | Duplicate ticket or missing link rejected | Duplicate (also padded) → `409 CONFLICT`, `details.ticket_number`; missing/`null`/`0`/negative/string/fractional link → `400`; unknown client, device, technician → `404`, `details.field`; device of another client → `409`, `details.field = device_id`; Receptionist or Administrator as assignee → `409`, `details.field = assigned_technician_id`; nothing stored | PASS |
| REQ-F-005 | Order list and item | `status`, `client_id`, `ticket_number` filters exact and combinable; lowercase/unknown status, `client_id=0`, empty `ticket_number`, unknown parameter → `400`; unknown id → `404` | PASS |
| REQ-F-017 / AC-F-017, BDD-015 | Administrator sets tier; change persists | `PUT …/loyalty-tier` Silver, Gold → `200 Client`; reopened `Gold`; audit rows Standard→Silver→Gold by the Administrator; same tier again writes nothing; invalid values → `400`; unknown client → `404` | PASS |
| REQ-F-017 / AC-F-017 | Authorized change persists under concurrent use | A tier change running at the same time as a `PATCH /clients/{id}` is silently reverted in 8 of 20 rounds; the audit log still shows the new tier | FAIL (D-1) |
| REQ-F-017 | Order spend alone does not change the tier | No code path writes `loyalty_tier` except `set_loyalty_tier`; order intake leaves the tier unchanged | PASS |
| BDD-011, REQ-F-002 | Technician (and Receptionist) cannot change a tier | `403 FORBIDDEN` for valid, invalid, `null` body, unknown and invalid client id; tier and audit unchanged | PASS |
| BDD-017, REQ-F-001 / AC-F-001 | No record contents without sign-in | 60 requests on all 13 operations without token, with an unknown token, with `Basic`, with valid, malformed or array body and invalid ids → `401 UNAUTHENTICATED`, body only `code`, `message`, no names, phones, serials or tickets; nothing changed | PASS |
| REQ-F-002 / AC-F-002, ADD-002 §1 | Every role reads and searches | Receptionist, Technician, Administrator → `200` on all 9 read and search requests, exact key sets | PASS |
| REQ-F-002, REQ-NF-002 | Technician cannot maintain clients, devices, orders | 22 Technician requests (valid, invalid, malformed, unknown id, invalid id) → `403`; Receptionist on the tier endpoint → `403`; records and audit unchanged | PASS |
| ADD-002 §2/§5, architecture § Request flow | Check order `401` → `403` → `400` → `404`/`409` | No token + bad body → `401`; denied role + bad body or unknown id → `403`; invalid body + unknown id → `400`; blank ticket + unknown client → `400`; unknown client + duplicate ticket → `404`; device of another client + duplicate ticket → `409 field=device_id` (link before uniqueness, as documented) | PASS |
| REQ-NF-002 / AC-NF-002 | Each change names one account | Every audit row has `actor_id`; Receptionist and Administrator changes carry their own ids | PASS |
| REQ-NF-003 / AC-NF-003 | Actor, time, record, field, old and new value | Client creation: `name`, `phone`, `email` (when set), `loyalty_tier`; device: 4 fields; order: `ticket_number`, `client_id`, `device_id`, `assigned_technician_id`, `status`, all with `previous_value = NULL`; updates one row per changed field | PASS |
| REQ-NF-003 | Audit equals the stored record | After concurrent updates, 2 client fields have a last audited value that differs from the stored value (`Alice 39` audited, `Alice 38` stored) | FAIL (D-1) |
| ADD-002 §4 | Integer cents | All `*_cents` are JSON integers; DB columns `bigint` with `>= 0` checks | PASS |
| ADD-002 §5 | Unified `ErrorResponse` | Every error body has only `code`, `message`, optional `details`; codes from the contract enum; `Content-Type: application/json` | PASS |

### Manual HTTP checks

| Request | Expected | Actual |
|---|---|---|
| Receptionist `POST /clients` (contract example) | `201`, keys `{id, name, phone, email, loyalty_tier}`, `Standard` | `201`, as expected |
| `POST /clients` without email / `email: null` | `201`, `email: null` | `201`, `email: null` |
| `POST /clients` `"  Dana Lee "`, `" +1-555-0300 "`, `" dana@lee.io "` | Trimmed | `Dana Lee`, `+1-555-0300`, `dana@lee.io` |
| `POST /clients` missing/empty/blank/`null`/numeric name or phone, unknown `loyalty_tier`/`id`, array, string | `400 VALIDATION_ERROR` | `400` (12 cases) |
| `POST /clients` email `bob`, `bob@`, `@vance.org`, `bob@vance`, `bob @vance.org`, `bob@@vance.org`, `bob@vance..org`, `""`, `" "` | `400`, `details.field = email` | `400`, `field=email` (9 cases) |
| `POST /clients` malformed JSON / `text/plain` / no body | `400` | `400` |
| `PATCH /clients/{id}` `{"phone"}`; `{"email": null}`; same name again | `200`; email cleared; no audit row | As expected |
| `PATCH /clients/{id}` `{}`, `name: null`, `phone: null`, `name: ""`, bad email, `loyalty_tier`, `id`, valid + `null` | `400` | `400` (8 cases) |
| `PATCH /clients/999999` valid / invalid body | `404` / `400` | `404` / `400` |
| `GET /clients/0`, `-1`, `abc`, `1.5`, `99999999999999999999`; `/clients/999999` | `400`; `404` | `400`; `404` |
| `GET /clients?q=bob`, `VANCE`, `555-01`, `0300`, `%25`, `_`, `nobody`, `a` | Name or phone, case-insensitive, literal wildcards | Matching sets as expected (`_` finds only the two names containing `_`) |
| `GET /clients?q=`, `?name=Bob`, `?q=a&q=b` | `400` | `400` |
| Administrator `PUT …/loyalty-tier` Silver, Gold, Gold again | `200`; 2 audit rows | `200`; 2 rows, actor = Administrator |
| `PUT …/loyalty-tier` `Platinum`, `gold`, `null`, `2`, `{}`, extra `name`; `/clients/999999/…`; `/clients/0/…` | `400`; `404`; `400` | As expected |
| `POST /devices` missing or `null` each of 4 fields; empty/blank serial or symptoms; `device_type` `laptop`/`Tablet`/`""`; `client_id` `0`, `-5`, `"1"`, `1.5`, overflow; extra `id` | `400` | `400` (21 cases, `details.field` where the domain rejects) |
| `POST /devices` unknown client; unknown client + empty serial | `404 field=client_id`; `400` | As expected |
| `POST /devices` second device with an existing serial for the same client | Not specified | `201` (serials are not unique) |
| `PATCH /devices/{id}` symptoms; `{}`, `client_id`, `null`/blank serial, `server`, `null` type, empty symptoms; `/devices/999999` | `200`; `400`; `404` | As expected |
| `GET /devices?client_id=`, `?q=thinkpad`, `?q=%25_`, both, `client_id=999999`; `client_id=0`/`abc`, `q=`, `serial_number=` | Filtered; `400` | As expected |
| `POST /orders` duplicate `FIX-2026-0001`, also `"  FIX-2026-0001  "` | `409 CONFLICT`, `details.ticket_number` | `409`, `FIX-2026-0001` |
| `POST /orders` unknown client / device / technician | `404`, `details.field` | `404`, `client_id` / `device_id` / `assigned_technician_id` |
| `POST /orders` device of another client; Receptionist or Administrator as assignee | `409 CONFLICT`, `details.field` | `409`, `device_id` / `assigned_technician_id` |
| `POST /orders` missing or `null` each field; links `0`, `-5`, `"1"`, `1.5`; extra `status`, `used_parts_cents`, `settlement_cents`, `id`; numeric ticket; malformed | `400` | `400` (26 cases) |
| `POST /orders` disabled Technician as assignee | Not specified (open point a) | `201` |
| `POST /orders` `fix-2026-0001` while `FIX-2026-0001` exists | Not specified (open point b) | `201` |
| `GET /orders`, `?status=Received`, `Diagnosing`, `client_id`, `ticket_number` exact, `FIX-2026` (prefix), combined | Filtered, ordered by id | As expected |
| `GET /orders?ticket_number=%20FIX-2026-0001` | Not specified | `200 []`; the filter is not trimmed although creation trims (O-1) |
| `GET /orders?status=received`, `Done`, `""`, `client_id=0`/`x`, `ticket_number=`, `device_id=1`, `q=x` | `400` | `400` |
| `GET /orders/0`, `-1`, `abc`, `1.5`; `/orders/999999` | `400 field=orderId`; `404` | As expected |
| `PATCH /orders/{id}` (P6 operation) | Not routed in P2 | `405`, empty body (as P1 O-2) |
| 40 rounds: `PATCH /clients/{id}` `{"name"}` and `{"phone"}` at the same time | Both changes kept | Lost one change in 40 of 40 rounds (D-1) |
| 20 rounds: `PUT …/loyalty-tier` and `PATCH /clients/{id}` `{"phone"}` at the same time | Tier kept | Tier reverted in 8 of 20 rounds; audit shows the new tier (D-1) |
| 20 rounds: two `PATCH /devices/{id}` on different fields at the same time | Both changes kept | Lost one change in 20 of 20 rounds (D-1) |

### Database checks

| Check | Expected | Actual | Result |
|---|---|---|---|
| `clients` | Checks on non-blank `name`/`phone`, email shape, tier enum | `clients_name_check`, `clients_phone_check`, `clients_email_check`, `clients_loyalty_tier_check`; `UPDATE` to `Platinum` or `nobody` refused | PASS |
| `devices` | FK to client, type enum, non-blank texts | `devices_client_id_fkey`, `devices_device_type_check`, `btrim` checks; `Tablet` refused; deleting a client with devices refused | PASS |
| `repair_orders` | Unique ticket, device belongs to client, cents `>= 0`, status enum, settlement only when terminal | `repair_orders_ticket_number_key`, composite `repair_orders_device_of_client_fkey`, 8 cents checks, status check, `check4`; direct inserts of a duplicate ticket, a foreign device, `-1` cents, `Done`, `Completed` without settlement, blank ticket all refused; moving a device to another client refused while an order references it | PASS |
| Indexes | Trigram GIN on name, phone, serial; B-tree on `client_id`, `status`, ticket | Present: `clients_name_trgm_idx`, `clients_phone_trgm_idx`, `devices_serial_number_trgm_idx`, `devices_client_id_idx`, `repair_orders_client_id_idx`, `repair_orders_status_idx`, `repair_orders_ticket_number_key` | PASS |
| `EXPLAIN ANALYZE` with 5,007 clients, 5,005 devices, 5,005 orders | Searches answer quickly; indexes usable | Client search (`name OR phone ILIKE`) and serial search: sequential scan, 2.4 ms and 1.5 ms; with scans disabled the planner uses `BitmapOr` over both trigram indexes (4.1 ms), so the indexes are usable but not cheaper at this size. `client_id` and `ticket_number` filters use their B-tree indexes (0.01–0.02 ms). The `$1 IS NULL OR …` form gives a sequential scan under a forced generic plan (0.3 ms). See limitations | PASS, note |
| Audit completeness | Every row has actor, time, record type, id, field | 250 rows, 0 incomplete; every client, device and order has its creation rows; every order exactly 5 | PASS |
| Audit equals record | Last audited value of each field = stored value | 2 client fields differ after the concurrent rounds (D-1) | FAIL (D-1) |
| `UPDATE audit_log` | Refused | `ERROR: audit_log is append-only` | PASS |

The generated rows for `EXPLAIN` were inserted directly in SQL after the HTTP checks, so they have no audit rows and are not counted above.

### Test review (risk R-11)

The tests assert contract values: statuses, `ErrorResponse.code`, `details.field` or `details.ticket_number`, exact key sets of `Client`, `Device` and `RepairOrder`, and the contract example values (`Bob Vance`, `SN-THINKPAD-9001`, `FIX-2026-0001`). Audit expectations are written out field by field, not copied from output. Every rejected or denied request is followed by a read-back of clients, devices, orders and the audit log. Domain tests cover the parsers, trimming, the intake rules and the audit changes. No test asserts an implementation value that contradicts the spec.

| Gap | Effect |
|---|---|
| No test of concurrent updates to one client or device | D-1 not caught |
| No test of an unknown link combined with a duplicate ticket (`404` before `409`), or an invalid body with an unknown id (`400` before `404`) | Verified manually above |
| Duplicate-ticket test does not check that a ticket differing only in letter case is accepted | Behaviour of open point b is unpinned; verified manually |
| No test pins the disabled-Technician assignee (open point a) | Verified manually: `201` |
| `ticket_number` list filter tested only with exact values, not padded ones | O-1 not visible |
| No test for overflowing integers in bodies or paths beyond the path id case | Verified manually: `400` |
| `GET /orders` and `GET /orders/{id}` are only tested on `Received` orders | Correct for P2; see open point d |

### Open points

| Point | Finding | Recommendation |
|---|---|---|
| (a) A disabled Technician can be assigned | Confirmed: `201`. The SRS and glossary say "assigned Technician" and that a disabled account cannot sign in; they say nothing about assigning one. Such an order can only be worked by an Administrator | Gate-owner decision, recorded as an addendum. Recommended: reject with `409 CONFLICT`, `details.field = assigned_technician_id`, the same answer as a non-Technician, because an assignee who cannot sign in is not a usable relationship. Not a defect against the current spec |
| (b) Ticket numbers compared exactly after trimming | Confirmed: `FIX-2026-0001` and `fix-2026-0001` coexist; padded duplicates are caught. The list filter is not trimmed (O-1) | Accept case-sensitive comparison unless the workshop's ticket format is case-insensitive; if it is, normalise the case on input (or index `lower(ticket_number)`) in an addendum. Trim the `ticket_number` filter the same way as creation |
| (c) Broken device or technician link uses `409 CONFLICT`, not `GUARD_VIOLATION` | The `createRepairOrder` `409` is described as "duplicate ticket number or invalid device/technician relationship"; ADD-002 §5 and the architecture reserve `GUARD_VIOLATION` for lifecycle and settlement guards. `details.field` tells the two `409` causes apart | Keep `CONFLICT`. Use the same code for the same relationship rules in P6 corrections |
| (d) `pricing::current_settlement` has an `unimplemented!` branch | Not reachable through any P2 endpoint: the only write to `repair_orders` is the intake `INSERT` with `Received`, and `correctRepairOrder` is not routed (`405`). Reached by setting one order to `Diagnosing` directly in SQL: `GET /orders/{id}` and unfiltered `GET /orders` drop the connection with no HTTP response, no `ErrorResponse` and no line in the JSON log (panic only on stderr); `?status=Received` still works. Order reverted afterwards | Acceptable for P2. P3 must replace the branch in the same change that introduces `Diagnosing`, otherwise one diagnosed order breaks every unfiltered order list. Make "no `unimplemented!` reachable from a routed handler" a P3 exit criterion; a panic-catching layer that answers `500 INTERNAL` and logs an `ERROR` line would also make such failures visible |

### Operational log

JSON-lines file `fixops.log.YYYY-MM-DD` in `FIXOPS_LOG_DIR`, 1,095 lines.

| Check | Expected | Actual | Result |
|---|---|---|---|
| Every request logged once | One `request finished` per answered request | 660 lines, 660 distinct `request_id`s. The 2 requests that hit the panic in open point d have no line | PASS, note |
| Level by status | `INFO` < 400, `WARN` 4xx, `ERROR` 5xx | 0 mismatches (433 `INFO`, 227 `WARN`); no `ERROR` lines | PASS |
| Actor and action | `actor_id`, `role`, `action` on authenticated P2 requests | Present on 558 of the 559 P2 requests not answered `401`; the missing one is the unrouted `PATCH /orders/{id}` (`405`). Query and body errors now carry `action` because the handlers authorize first (P1 O-1 is resolved for the new endpoints) | PASS |
| Denials | `action denied` per role denial | 26 lines = 26 `403`, each with actor, role, action | PASS |
| Rejections | `request rejected` with `code` per 4xx | 226 lines = 227 4xx minus the `405` | PASS |
| Changes | `record changed` with field names only | 181 lines (Client 123, Device 46, RepairOrder 5, StaffAccount 7) | PASS |
| Secrets | No password, hash or token | 0 hits for both passwords, the issued admin token, `$argon2`, `Bearer`, `Authorization`; no query strings | PASS |
| Record values | None | 0 hits for client names, phones, emails, serials, symptoms. The ticket number appears twice, in `details` of the duplicate-ticket rejection (O-2) | PASS, note |

### Defects

| ID | Severity | Requirement | Description |
|---|---|---|---|
| D-1 | Medium | REQ-F-017 / AC-F-017, REQ-F-003, REQ-F-004, REQ-NF-003 | Concurrent updates of one client or device lose changes, and the audit log no longer matches the record. `update_client`, `set_loyalty_tier` and `update_device` load the record, apply the patch and write **every** column back (`UPDATE clients SET name, phone, email, loyalty_tier …`, `crates/infra/src/clients.rs`; same in `devices.rs`) with no version check or row lock. The second writer restores the first writer's fields to their old values, and that restore is not audited. Measured: 40/40 lost changes for name + phone, 20/20 for two device fields, 8/20 reverted tier changes when a tier change meets a phone edit; afterwards the audit log shows `Alice 39` and tier `Gold` while the record holds `Alice 38` or the old tier. Expected: an authorized change persists and the audit log reflects the stored values. A `version` check like the one documented for orders (`409` on stale), a `SELECT … FOR UPDATE` in the transaction, or writing only the changed columns would close it |

| ID | Kind | Note |
|---|---|---|
| O-1 | Observation | `GET /orders?ticket_number=` is compared without trimming, while creation trims; `%20FIX-2026-0001` finds nothing. The contract does not say; trim for consistency |
| O-2 | Observation | `request rejected` logs `details`, so the duplicate ticket number appears in the operational log. It is an identifier the caller sent, not personal data, and P1 does the same with `details.username`; the architecture forbids changed values, which this is not. Accept, or log only `details` keys |
| O-3 | Observation | No maximum length on name, phone, email, serial, symptoms or ticket number; only the 2 MB body limit applies. The spec is silent |

### Limitations (run 1)

- Performance (REQ-NF-001, 10 concurrent users, 10,000 orders) is P7. With about 5,000 rows per table every search answered in under 3 ms; the planner preferred sequential scans to the trigram indexes at that size. `GET /clients`, `/devices`, `/orders` return all matches without paging; the contract defines none.
- The `$1 IS NULL OR …` filter form cannot use an index under a generic plan. PostgreSQL keeps custom plans when they are much cheaper, so this did not show here; it should be part of the P7 load test.
- BDD-015 was checked only for tier persistence and audit. The settlement effect of a tier change is P5.
- The `unimplemented!` branch was reached only by editing the database directly; no P2 endpoint can reach it.
- The concurrency results depend on timing; the counts show the race exists, not its rate in normal use.
- The service still connects as the database superuser (accepted in P1 and moved to P7).

## Run 2

**Result:** PASS · **Code:** `fix-32/intake-fixes` = run 1 log, then `test: cover concurrent client and device updates`, `fix: reject stale client and device updates`, on top of current `main` · **Gate-owner decisions:** fix D-1; dispositions of the open points and observations are listed under [Limitations](#limitations)

### Commands

| Command | Result |
|---|---|
| `cargo fmt --all --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo test --workspace` | 138 passed, 0 failed: `intake` 35, `stale_updates` 5 (new), `staff_access` 32, `logging` 4, `permissions` 3, `contract` 3, `platform` 2, `fixops-domain` 39, `fixops-app` 8, `fixops-infra` 4, `fixops` lib 3 |
| `cargo run --release` on a fresh database, same settings as run 1 | Migrations ran (including `client_device_version`), 856 requests served, no `ERROR` lines, no `5xx` |
| Run 1 HTTP script, unchanged | 268 checks: 267 as in run 1; the 1 former mismatch now ends in `409` instead of a silent loss (see below) |
| Concurrency script (per round: both responses, record read-back, audit compared with the record) | 80 rounds, 0 silent losses, 0 audit/record mismatches |
| Server stopped, database dropped | Done |

### Review of the test and fix commits

| Item | Finding | Result |
|---|---|---|
| `stale_client_update_is_rejected_and_changes_nothing`, `stale_tier_change_is_rejected_and_changes_nothing`, `stale_device_update_is_rejected_and_changes_nothing` | Build a stale save deterministically (read through the store, change through the API, save the old copy). Assert `StoreError::Stale` → `409 GUARD_VIOLATION`, the record read back unchanged with the newer value kept, and the audit log unchanged. Expresses AC-F-017 ("an authorized change persists") and REQ-NF-003 (no unaudited change) without depending on timing | OK |
| `fresh_saves_succeed_one_after_another` | Re-reading before each save succeeds every time, so the check does not block normal sequential work | OK |
| `sequential_updates_persist_and_match_the_audit_log` | After API updates of every client and device field and a tier change, the last audited value of each field equals the stored value. Directly the invariant D-1 broke | OK |
| Gaps | The stale path is exercised at the store, and its `409` through `ApiError` conversion, not through a live HTTP race; no test retries through HTTP after a `409`. Both are covered by the measurement below | Note |
| Fix | Migration adds `version bigint NOT NULL DEFAULT 0 CHECK (version >= 0)` to `clients` and `devices`. Updates read record and version in one statement and write `… version = version + 1 WHERE id = $1 AND version = $2`; zero rows → `StoreError::Stale` before the audit insert, and the dropped transaction rolls back. `update_client`, `set_loyalty_tier` and `update_device` all go through it; a no-op update writes nothing and needs no check | OK |

### Concurrency measurement (former D-1)

Same rounds as run 1, release build. Each round sends two requests on different fields of one record at the same time, then reads the record and its audit entries back.

| Rounds | Run 1 | Run 2 responses | Applied but lost | Audit ≠ record |
|---|---|---|---|---|
| 40 × `PATCH /clients/{id}` name + phone | 40 rounds lost a change silently | 40 × `200`, 40 × `409 GUARD_VIOLATION` | 0 | 0 |
| 20 × `PUT …/loyalty-tier` + `PATCH` phone | 8 tier changes reverted silently | 24 × `200`, 16 × `409 GUARD_VIOLATION` | 0 | 0 |
| 20 × `PATCH /devices/{id}` serial + symptoms | 20 rounds lost a change silently | 20 × `200`, 20 × `409 GUARD_VIOLATION` | 0 | 0 |

- Every request either applied (its value is in the record after the round) or got `409` with only `code`, `message`, `details`; no other status occurred.
- After all rounds, SQL comparison of the last audited value of every client and device field with the stored value: 0 differences (run 1: 2).
- A `409`'d change resent alone → `200`, applied.
- `version` ranges 0–41 on clients and 0–20 on devices, one increment per applied change.
- The run 1 HTTP script's own concurrency check still reports 40 "lost" rounds because it counts a rejected request as lost; in run 2 all 40 are `409` responses (116 `GUARD_VIOLATION` lines in the log = 40 + 40 + 16 + 20), not silent losses.

### Spot checks of run 1 results

All 267 run 1 expectations hold with identical results: 401 on all 13 operations, 403 for denied roles whatever the request, check order, validation, `404`/`409` link answers, duplicate ticket, open points a and b, the read-back after every rejection. Status counts: `200` 486, `201` 27, `400` 123, `401` 60, `403` 26, `404` 11, `405` 1, `409` 122 (`CONFLICT` 6, `GUARD_VIOLATION` 116).

### Operational log

JSON-lines file, 1,378 lines.

| Check | Actual | Result |
|---|---|---|
| Every request logged once | 856 `request finished`, 856 distinct `request_id`s | PASS |
| Level by status | 0 mismatches; no `ERROR` lines | PASS |
| Actor and action | On 677 of 678 P2 requests not answered `401`; the exception is the unrouted `PATCH /orders/{id}` (`405`) | PASS |
| Denials | 26 `action denied` = 26 `403` | PASS |
| Rejections | 342 `request rejected` = 343 4xx minus the `405`; each stale save logged as `GUARD_VIOLATION` with `action` `updateClient`, `setClientLoyaltyTier` or `updateDevice` and empty `details` | PASS |
| Changes | 151 `record changed` (Client 113, Device 26, RepairOrder 5, StaffAccount 7); no line for a rejected stale save | PASS |
| Secrets and record values | 0 hits for passwords, the token, `$argon2`, `Bearer`, `Authorization`, names, phones, emails, serials, symptoms; the ticket number twice in duplicate-ticket `details` (O-2, accepted) | PASS |

### Defects

None. D-1 from run 1 is fixed and re-verified.

| ID | Kind | Note |
|---|---|---|
| O-4 | Observation | `openapi.yaml` lists no `409` for `updateClient`, `setClientLoyaltyTier` and `updateDevice`, which can now answer `409 GUARD_VIOLATION`. ADD-002 §5 allows `409` for a violated guard, so the status is within the policy, but the operations' response lists should name it; candidate contract addendum, together with the choice of `GUARD_VIOLATION` (rather than `CONFLICT`) for a stale save |
| O-5 | Observation | Architecture § Concurrency still describes the `version` check for orders only; it now applies to clients and devices too |

## Limitations

Run 1 limitations still apply (performance and paging are P7; the generic-plan note for the `$1 IS NULL OR …` filters; BDD-015 settlement effect is P5; the service connects as superuser, moved to P7). In addition:

- The version is not exposed in the API, so the check protects concurrent server-side saves, not a screen that was loaded earlier: two staff who edit the **same** field one after another still get "last save wins", with both saves audited. The contract has no field for a client-side version.
- Concurrency results depend on timing; they show that no interleaving in 80 rounds lost a change, not a proof for every interleaving. The deterministic stale tests cover the mechanism.

Gate-owner dispositions:

| Point | Disposition |
|---|---|
| (a) Assigning a disabled Technician | Accepted for now; the spec is silent; candidate addendum |
| (b) Ticket numbers compared exactly after trimming | Accepted |
| O-1 `ticket_number` list filter not trimmed | Moved to the refactor backlog; order files are being changed by another package |
| (c) `409 CONFLICT` for a broken device or technician link | Kept |
| (d) `unimplemented!` branch in `pricing::current_settlement` | Removed in P3 |
| O-2 Ticket number in log `details` | Accepted, no change |
| O-3 No maximum text lengths | Accepted, no change |
