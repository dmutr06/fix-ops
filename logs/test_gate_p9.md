# TEST-GATE P9: Staff web app

**Decision:** GO (delegated; human review follows) · **Ticket:** [FIX-63](https://linear.app/fixops/issue/FIX-63) · **Scope:** REQ-F-021, REQ-F-022, REQ-F-023, REQ-NF-001 · **Baseline:** [SRS 1.1.0](../spec/srs.md), [ADD-004](../spec/addenda/ADD-004-web-interface.md), [concept](../spec/concept-web.md)

Playwright tests in `web/e2e/repair.spec.ts`, written before the screens, run on a desktop and a 360px phone viewport against the real service on a fresh database.

| Requirement | Source | Test |
|---|---|---|
| REQ-F-021, REQ-NF-001 | AC-F-021, AC-NF-001, BDD-025 | `BDD-025: three roles take an order from intake to Completed in the browser`: staff, client, device, intake, diagnosis, estimate, approval, repair, blocked Ready, payment, pickup, completion, history; each step within 2 s; Staff and Cancel not offered where the role may not |
| REQ-F-023 | AC-F-023, BDD-023 | Same test: orders reached through the Technician's and Receptionist's queues |
| REQ-F-022 | AC-F-022, BDD-022 | `BDD-022: signing out ends the browser session`; the session survives a reload |
| REQ-F-001 | AC-F-001 | `a wrong password shows the service's message` |

Baseline: all six runs fail because the service serves no sign-in page yet.
