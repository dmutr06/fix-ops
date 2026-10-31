# ADD-002: REST API read permissions and contract conventions

**Changes:** SRS 1.0.1 → 1.0.2 · **Affects:** REQ-F-001–005, REQ-F-012, REQ-F-013, REQ-F-018, REQ-F-019, REQ-NF-001–003

## Policy

1. Any signed-in staff member (`Receptionist`, `Technician`, `Administrator`) may read and search clients, devices, repair orders, and audit entries (`GET /clients`, `GET /devices`, `GET /orders`, `GET /audit-logs`, and item `GET` endpoints). Unauthenticated requests return `401`.
2. Viewing and managing staff accounts (`GET /staff`, `POST /staff`, `PATCH /staff/{staffId}`) is restricted to `Administrator` (`403` for other roles).
3. On `Ready` orders, payment (`PUT /orders/{orderId}/payment`) and device pickup (`PUT /orders/{orderId}/pickup`) are recorded as independent sub-resources; `POST /orders/{orderId}/complete` transitions `Ready → Completed` only when both full settlement payment and pickup are present (`BDD-010`, `BDD-012`).
4. All monetary fields in API payloads use whole USD cents (`*_cents >= 0`). Display formatting (`$X.XX`) is a client concern.
5. Every rejected operation leaves stored records unchanged and returns `ErrorResponse` (`code`, `message`, optional `details`) with status `400` (invalid parameters or body), `401` (missing/invalid credentials or session), `403` (disabled account or role denied), `404` (unknown record), or `409` (duplicate identifier or violated lifecycle/settlement guard).

## Test impact

| Requirements | What to test | Scenarios |
|---|---|---|
| REQ-F-001, REQ-F-002, REQ-F-019, REQ-NF-002 | Unauthenticated `401`; disabled login `403`; role-denied `403` on mutations and `/staff`; signed-in staff `200` on reads/search. | BDD-011, 017, 018, 019, 020 |
| REQ-F-003–018, REQ-NF-001, REQ-NF-003 | Integer-cent schemas; `400` on invalid/missing fields; decoupled pickup/payment with `409` on incomplete guards; `409` on duplicate ticket or terminal reopen; unified `ErrorResponse`. | BDD-001–016 |
