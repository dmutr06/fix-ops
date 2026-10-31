# Traceability matrix

IDs refer to [`srs.md`](srs.md) and [`openapi.yaml`](openapi.yaml). Packages and their Test / Feature / Verification tickets are listed in the [delivery plan](../docs/delivery_plan.md#work-packages). Every requirement is also covered by PLAN-GATE (FIX-26), [API-GATE](../logs/api_gate.md), and RELEASE-GATE (FIX-51).

| Requirement | AC | BDD | API Operations (`openapi.yaml`) | Package | Tickets | Addendum |
|---|---|---|---|---|---|---|
| REQ-F-001 | AC-F-001 | 017 | `createSession`; `bearerAuth` on all 25 protected operations | P1 | FIX-27, 28, 29 | ADD-002 |
| REQ-F-002 | AC-F-002 | 011, 018, 019, 020 | `x-roles` guards on all 25 protected operations | P1 | FIX-27, 28, 29 | ADD-001, ADD-002 |
| REQ-F-003 | AC-F-003 | 001, 002 | `listClients`, `createClient`, `getClient`, `updateClient` | P2 | FIX-30, 31, 32 | ADD-002 |
| REQ-F-004 | AC-F-004 | 001, 002 | `listDevices`, `createDevice`, `getDevice`, `updateDevice` | P2 | FIX-30, 31, 32 | ADD-002 |
| REQ-F-005 | AC-F-005 | 001, 002 | `listRepairOrders`, `createRepairOrder`, `getRepairOrder` | P2 | FIX-30, 31, 32 | ADD-002 |
| REQ-F-006 | AC-F-006 | 006, 013 | `recordDiagnosis` | P3 | FIX-33, 34, 35 | |
| REQ-F-007 | AC-F-007 | 003, 006 | `recordEstimate` | P3 | FIX-33, 34, 35 | |
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
| REQ-F-019 | AC-F-019 | — | `createSession`, `listStaffAccounts`, `createStaffAccount`, `updateStaffAccount` | P1 | FIX-27, 28, 29 | ADD-002 |
| REQ-NF-001 | AC-NF-001 | — | `listClients`, `getClient`, `listDevices`, `getDevice`, `listRepairOrders`, `createRepairOrder`, `getRepairOrder` (2s latency SLA across routine actions) | P7 | FIX-45, 46, 47 | ADD-002 |
| REQ-NF-002 | AC-NF-002 | 011, 018, 019, 020 | `createSession`; `bearerAuth` and `x-roles` on all 25 protected operations | P1 | FIX-27, 28, 29 | ADD-001, ADD-002 |
| REQ-NF-003 | AC-NF-003 | 016 | `listAuditLogs`; audit trail on all 17 state-changing operations | P1, P6 | FIX-27, 28, 29, 42, 43, 44 | ADD-001, ADD-002 |
| REQ-NF-004 | AC-NF-004 | — | — (daily database backup and restore; infrastructure operation) | P7 | FIX-45, 46, 47 | |

`—` means no dedicated scenario or endpoint; the acceptance criterion still applies.
