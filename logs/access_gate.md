# ACCESS-GATE

**Decision:** GO · **Ticket:** [FIX-48](https://linear.app/fixops/issue/FIX-48) · **Scope:** REQ-F-001, REQ-F-002, REQ-F-019, REQ-NF-002 ([delivery plan](../docs/delivery_plan.md#gates)) · **Evidence:** [TEST-GATE P1](test_gate_p1.md), [verification P1](verification_p1.md), [TEST-GATE logging](test_gate_logging.md)

Sign-in and role restrictions work, and a denied action changes nothing. P2–P7 may start.

## Evidence

| Check | Result |
|---|---|
| Tests written from the SRS and contract before code; red for the expected reason | 43 red on `not yet implemented`, then green ([TEST-GATE P1](test_gate_p1.md)) |
| Independent verification, run 1 | FAIL: D-1 sign-in timing revealed usernames (Medium); D-2 denied role got 400 before 403 (Low); D-3 `null` accepted on non-nullable fields (Low) |
| Defect fixes, test-first | 4 new tests red for the expected reasons, then green |
| Independent verification, run 2 | PASS: 74/74 tests, 117/121 HTTP checks as expected (4 notes, no mismatches), sign-in median 11.9 ms wrong password vs 11.7 ms unknown user (release) |
| Denied actions change nothing | Read-back of records and audit log after every denied role and request |
| Audit trail | Actor, time, record, field, old and new value on every change; `UPDATE`, `DELETE` and `TRUNCATE` on `audit_log` refused |
| Operational log | Every request with id, actor, role, action and status; no password, hash or token |

## Accepted limitations

| Item | Disposition |
|---|---|
| The service connects as a database superuser | Least-privilege role in P7 (operations) |
| Nothing stops disabling the last Administrator | The spec is silent; candidate addendum |
| Usernames are case-sensitive | The spec is silent; candidate addendum |
| `405 Method Not Allowed` has an empty body and no log line | Refactor backlog |
| `GET /audit-logs` validates filters before the role check, so that log line has no `action` | Responses correct; refactor backlog |
| BDD-011 and BDD-017 for clients and orders | Covered by P2, when those routes exist |

**Next:** client and order intake tests (FIX-30).
