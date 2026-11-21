# ADD-004: Staff web interface

**Changes:** SRS 1.0.3 → 1.1.0 · **Concept:** [concept-web.md](../concept-web.md) · **Affects:** REQ-F-001, REQ-F-002, REQ-NF-001; adds REQ-F-021–023

## Policy

1. **Web interface.** Staff do every action of §2 in a browser app served by the service at `/`. It shows only the actions the signed-in role may take and shows the service's message when a step is rejected. It adds no rule of its own.
2. **Browser session.** Signing in also sets a `fixops_session` cookie (`HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, 12 hours). The service accepts the cookie or the bearer token. `GET /sessions/current` returns the signed-in account; `DELETE /sessions/current` ends the session and clears the cookie.
3. **Staff list.** Every role may read the staff list (`listStaffAccounts`), filtered by `role` and `enabled`, so Receptionists can choose a Technician and audit entries can show names. Creating and changing accounts stays with the Administrator.
4. **Work queues.** `listRepairOrders` also filters by `assigned_technician_id`, `open=true` (not `Completed` or `Cancelled`) and `awaiting_decision=true` (estimate recorded, no client decision). After sign-in the Receptionist sees orders awaiting a decision and `Ready` orders, the Technician sees their open assigned orders, and the Administrator sees all open orders.
5. **Paging.** Every list operation accepts `limit` (1–100) and `offset` (0 or more) and returns the total match count in `X-Total-Count`. Without `limit` it returns every match, as before.

## Test impact

| Requirements | What to test | Scenarios |
|---|---|---|
| REQ-F-001, REQ-F-022 | Sign-in sets the cookie; the cookie authenticates; current session; sign-out ends the session and a later request is `401` | BDD-022 |
| REQ-F-002, REQ-NF-002 | Every role reads staff with `role`/`enabled` filters; only the Administrator creates or updates | BDD-024 |
| REQ-F-023 | Queue filters and paging return the right orders and `X-Total-Count`; invalid values are `400` | BDD-023 |
| REQ-F-021, REQ-NF-001 | In a browser, each role signs in, sees its queue and completes its steps; 95% of routine actions show their result within 2 s | BDD-025 |
