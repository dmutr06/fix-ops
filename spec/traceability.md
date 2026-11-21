# Traceability matrix

IDs refer to [`srs.md`](srs.md) and [`openapi.yaml`](openapi.yaml). Packages and their Test / Feature / Verification tickets are listed in the [delivery plan](../docs/delivery_plan.md#work-packages). Every requirement is also covered by PLAN-GATE (FIX-26), [API-GATE](../logs/api_gate.md), and RELEASE-GATE (FIX-51).

| Requirement | AC | BDD | API Operations (`openapi.yaml`) | Package | Tickets | Addendum |
|---|---|---|---|---|---|---|
| REQ-F-001 | AC-F-001 | 017, 022 | `createSession`, `getCurrentSession`, `deleteSession`; `bearerAuth` on all 27 protected operations | P1, P8 | FIX-27, 28, 29, 62 | ADD-002, ADD-004 |
| REQ-F-002 | AC-F-002 | 011, 018, 019, 020, 024 | `x-roles` guards on all 27 protected operations | P1, P8 | FIX-27, 28, 29, 62 | ADD-001, ADD-002, ADD-004 |
| REQ-F-003 | AC-F-003 | 001, 002 | `listClients`, `createClient`, `getClient`, `updateClient` | P2 | FIX-30, 31, 32 | ADD-002 |
| REQ-F-004 | AC-F-004 | 001, 002 | `listDevices`, `createDevice`, `getDevice`, `updateDevice` | P2 | FIX-30, 31, 32 | ADD-002 |
| REQ-F-005 | AC-F-005 | 001, 002 | `listRepairOrders`, `createRepairOrder`, `getRepairOrder` | P2 | FIX-30, 31, 32 | ADD-002 |
| REQ-F-006 | AC-F-006 | 013 | `recordDiagnosis` | P3 | FIX-33, 34, 35 | |
| REQ-F-007 | AC-F-007 | 003 | `recordEstimate` | P3 | FIX-33, 34, 35 | |
| REQ-F-008 | AC-F-008 | 003, 004, 006 | `recordClientDecision` | P3 | FIX-33, 34, 35 | |
| REQ-F-009 | AC-F-009 | 003, 004 | `updateRepairWork` | P3 | FIX-33, 34, 35 | |
| REQ-F-010 | AC-F-010 | 007 | `updateRepairWork` | P4 | FIX-36, 37, 38 | |
| REQ-F-011 | AC-F-011 | 009 | `updateRepairWork` | P4 | FIX-36, 37, 38 | |
| REQ-F-012 | AC-F-012 | 010, 012 | `recordPayment` | P5 | FIX-39, 40, 41 | ADD-002 |
| REQ-F-013 | AC-F-013 | 010, 012 | `recordPickup`, `completeRepairOrder` | P5 | FIX-39, 40, 41 | ADD-002 |
| REQ-F-014 | AC-F-014 | 004, 005, 006, 009, 010, 018, 019 | `recordDiagnosis`, `updateRepairWork`, `completeRepairOrder`, `cancelRepairOrder`, `correctRepairOrder` | P4 | FIX-36, 37, 38 | ADD-001 |
| REQ-F-015 | AC-F-015 | 005, 006, 007, 013 | `listRepairOrders`, `getRepairOrder`, `cancelRepairOrder`, `correctRepairOrder` | P5 | FIX-39, 40, 41 | |
| REQ-F-016 | AC-F-016 | 007, 008, 014, 015 | `setClientLoyaltyTier`, `listRepairOrders`, `getRepairOrder`, `updateRepairWork`, `completeRepairOrder`, `cancelRepairOrder`, `correctRepairOrder` | P5 | FIX-39, 40, 41 | |
| REQ-F-017 | AC-F-017 | 011, 015 | `setClientLoyaltyTier` | P2 | FIX-30, 31, 32 | |
| REQ-F-018 | AC-F-018 | 016 | `correctRepairOrder`, `listAuditLogs` | P6 | FIX-42, 43, 44 | ADD-001, ADD-002 |
| REQ-F-019 | AC-F-019 | 021 | `createSession`, `listStaffAccounts`, `createStaffAccount`, `updateStaffAccount` | P1 | FIX-27, 28, 29 | ADD-002 |
| REQ-F-020 | AC-F-020 | 021 | `updateStaffAccount` | ADD-003 | FIX-54 | ADD-003 |
| REQ-F-021 | AC-F-021 | 025 | all operations, through the browser interface | P9 | FIX-63 | ADD-004 |
| REQ-F-022 | AC-F-022 | 022 | `createSession`, `getCurrentSession`, `deleteSession`; `cookieAuth` | P8 | FIX-62 | ADD-004 |
| REQ-F-023 | AC-F-023 | 023 | `listRepairOrders`, `listClients`, `listDevices`, `listStaffAccounts`, `listAuditLogs` | P8, P9 | FIX-62, 63 | ADD-004 |
| REQ-NF-001 | AC-NF-001 | 025 | `listClients`, `getClient`, `listDevices`, `getDevice`, `listRepairOrders`, `createRepairOrder`, `getRepairOrder` (2s latency SLA across routine actions) | P7, P9 | FIX-45, 46, 47, 63 | ADD-002, ADD-004 |
| REQ-NF-002 | AC-NF-002 | 011, 018, 019, 020, 024 | `createSession`; `bearerAuth` and `x-roles` on all 27 protected operations | P1, P8 | FIX-27, 28, 29, 62 | ADD-001, ADD-002, ADD-004 |
| REQ-NF-003 | AC-NF-003 | 016 | `listAuditLogs`; audit trail on all 17 state-changing operations | P1, P6 | FIX-27, 28, 29, 42, 43, 44 | ADD-001, ADD-002 |
| REQ-NF-004 | AC-NF-004 | — | — (daily database backup and restore; infrastructure operation) | P7 | FIX-45, 46, 47 | |

`—` means no dedicated scenario or endpoint; the acceptance criterion still applies.

## Implementation

Test files are in `tests/` unless a crate path is given; `domain/` and `app/` are `crates/domain/src/` and `crates/app/src/`. `UG` is the [user guide](../docs/user_guide.md), `ARCH` the [architecture](../docs/architecture.md). `tests/traceability.rs` checks both tables against the SRS, the contract and the files they name.

| Requirement | Tests | Code | Docs |
|---|---|---|---|
| REQ-F-001 | `staff_access.rs`, `intake.rs`, `web_api.rs` | `app/staff.rs`, `src/http/auth.rs` | UG [Web app](../docs/user_guide.md#web-app), [Sign in](../docs/user_guide.md#sign-in-api); ARCH [Sessions](../docs/architecture.md#sessions) |
| REQ-F-002 | `permissions.rs`, `staff_access.rs`, `diagnosis.rs`, `settlement.rs`, `web_api.rs`, `domain/staff.rs` | `domain/staff.rs`, `app/lib.rs` | UG [Who does what](../docs/user_guide.md#who-does-what); ARCH [Request flow](../docs/architecture.md#request-flow) |
| REQ-F-003 | `intake.rs`, `stale_updates.rs`, `domain/client.rs` | `app/clients.rs`, `domain/client.rs` | UG [Who does what](../docs/user_guide.md#who-does-what) |
| REQ-F-004 | `intake.rs`, `stale_updates.rs`, `domain/device.rs` | `app/devices.rs`, `domain/device.rs` | UG [Who does what](../docs/user_guide.md#who-does-what) |
| REQ-F-005 | `intake.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [A repair, step by step](../docs/user_guide.md#a-repair-step-by-step); ARCH [Intake](../docs/architecture.md#intake) |
| REQ-F-006 | `diagnosis.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [A repair, step by step](../docs/user_guide.md#a-repair-step-by-step) |
| REQ-F-007 | `diagnosis.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [A repair, step by step](../docs/user_guide.md#a-repair-step-by-step) |
| REQ-F-008 | `diagnosis.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [A repair, step by step](../docs/user_guide.md#a-repair-step-by-step) |
| REQ-F-009 | `diagnosis.rs`, `repair.rs`, `domain/order.rs` | `domain/order.rs` | UG [Errors](../docs/user_guide.md#errors) |
| REQ-F-010 | `repair.rs`, `settlement.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [A repair, step by step](../docs/user_guide.md#a-repair-step-by-step) |
| REQ-F-011 | `repair.rs`, `domain/order.rs` | `domain/order.rs` | UG [Errors](../docs/user_guide.md#errors) |
| REQ-F-012 | `settlement.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [Money](../docs/user_guide.md#money) |
| REQ-F-013 | `settlement.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [Money](../docs/user_guide.md#money) |
| REQ-F-014 | `diagnosis.rs`, `repair.rs`, `settlement.rs`, `domain/order.rs` | `domain/order.rs` | UG [A repair, step by step](../docs/user_guide.md#a-repair-step-by-step) |
| REQ-F-015 | `settlement.rs`, `domain/pricing.rs` | `domain/order.rs`, `domain/pricing.rs` | UG [Money](../docs/user_guide.md#money); ARCH [Settlement](../docs/architecture.md#settlement) |
| REQ-F-016 | `settlement.rs`, `repair.rs`, `domain/pricing.rs` | `domain/pricing.rs` | UG [Money](../docs/user_guide.md#money); ARCH [Settlement](../docs/architecture.md#settlement) |
| REQ-F-017 | `intake.rs`, `stale_updates.rs` | `app/clients.rs`, `domain/client.rs` | UG [Who does what](../docs/user_guide.md#who-does-what) |
| REQ-F-018 | `corrections.rs`, `domain/order.rs` | `app/orders.rs`, `domain/order.rs` | UG [Who does what](../docs/user_guide.md#who-does-what) |
| REQ-F-019 | `staff_access.rs`, `domain/staff.rs` | `app/staff.rs`, `domain/staff.rs` | UG [Staff accounts](../docs/user_guide.md#staff-accounts-administrator) |
| REQ-F-020 | `staff_access.rs` | `app/staff.rs`, `crates/infra/src/staff.rs` | UG [Staff accounts](../docs/user_guide.md#staff-accounts-administrator) |
| REQ-F-021 | `web/e2e/repair.spec.ts` | `web/src/App.tsx` | UG [Web app](../docs/user_guide.md#web-app) |
| REQ-F-022 | `web_api.rs`, `web/e2e/repair.spec.ts` | `src/http/sessions.rs`, `src/http/auth.rs` | UG [Web app](../docs/user_guide.md#web-app); ARCH [Sessions](../docs/architecture.md#sessions) |
| REQ-F-023 | `web_api.rs`, `web/e2e/repair.spec.ts` | `app/orders.rs`, `src/http/params.rs` | UG [Web app](../docs/user_guide.md#web-app) |
| REQ-NF-001 | `scripts/check-performance.py`, `web/e2e/repair.spec.ts` | `crates/infra/migrations/20261109000001_intake.sql` | [Performance](../docs/performance.md) |
| REQ-NF-002 | `permissions.rs`, `staff_access.rs`, `web_api.rs` | `domain/staff.rs` | UG [Who does what](../docs/user_guide.md#who-does-what) |
| REQ-NF-003 | `intake.rs`, `staff_access.rs`, `diagnosis.rs`, `corrections.rs`, `stale_updates.rs`, `domain/audit.rs` | `domain/audit.rs`, `crates/infra/src/audit.rs` | UG [Audit log](../docs/user_guide.md#audit-log); ARCH [Audit contract](../docs/architecture.md#audit-contract) |
| REQ-NF-004 | `tests/recovery.sh` | `scripts/backup.sh`, `scripts/restore.sh` | [Recovery](../docs/recovery.md) |

Tests that check the contract and operations rather than one requirement: `contract.rs` and `platform.rs` (contract, ADD-002), `logging.rs` (operational log, FIX-53).
