# Staff workflow validation

**Ticket:** [FIX-50](https://linear.app/fixops/issue/FIX-50) · **Protocol:** [docs/validation.md](../docs/validation.md) · **Result:** PASS

The happy path and BDD-001–020 were run against the API on a disposable local PostgreSQL database, as the Administrator, Receptionist and Technician roles. After every denial or failed guard the order was read back unchanged.

| Scenario | Result |
|---|---|
| Happy path: Standard tier, $100.00 parts and labor | `Completed`; discount 0; settlement 20000 cents; payment and pickup kept |
| BDD-001 intake | `Received`; client, device and technician linked |
| BDD-002 missing serial, duplicate ticket | `400 VALIDATION_ERROR`; `409 CONFLICT`; nothing created |
| BDD-003 approval then `InProgress` | `200`; decision and timestamp readable |
| BDD-004 `InProgress` without approval | `409 GUARD_VIOLATION`; stays `Diagnosing` |
| BDD-005 cancel `Received` | `Cancelled`; settlement 0 |
| BDD-006 cancel after declined estimate | settlement 3000 |
| BDD-007 Gold, cancel in `InProgress` | discount 1000; settlement 14000 |
| BDD-008 Silver, `Ready` | discount 1000; settlement 19000 |
| BDD-009 `Ready` without repair check | `409`, then `200` once checked |
| BDD-010 complete before pickup | `409`, then `Completed` after pickup |
| BDD-011 Technician sets loyalty tier | `403 FORBIDDEN`; tier unchanged |
| BDD-012 partial payment | `409`; stays `Ready` |
| BDD-013 cancel without findings | settlement 0 |
| BDD-014 half-cent labor, Silver | discount 1; settlement 4 |
| BDD-015 tier changed to Gold before final settlement | discount 2000; settlement 18000 |
| BDD-016 labor correction | Persisted 8000; audit shows actor, time, field, old and new value |
| BDD-017 no token | `401` for client and order |
| BDD-018 cancel in three stages, Receptionist and Administrator | all `200`; 0, 3000 and incurred cost |
| BDD-019 Technician cancels | `403`; order and audit history unchanged |
| BDD-020 Administrator actions and guards | Client and diagnosis succeed; unapproved `InProgress` `409`; reopening `Completed` `409`; ticket correction on a `Completed` order audited by the Administrator; labor correction that breaks the paid amount rejected with `409` |

Operational log: 204 rejection, denial and change entries recorded; no bearer token appears in any log line.

Not covered: browser workflow and timing (no frontend), and daily backup observation (see [FINANCIAL-DATA-GATE](financial_data_gate.md)).
