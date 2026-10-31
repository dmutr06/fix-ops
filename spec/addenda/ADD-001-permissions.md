# ADD-001: Cancellation and Administrator permissions

**Changes:** SRS 1.0.0 → 1.0.1 · **Ticket:** [FIX-24](https://linear.app/fixops/issue/FIX-24) · **Affects:** REQ-F-002, REQ-F-014, REQ-F-018, REQ-NF-002, REQ-NF-003

## Policy

1. Receptionist and Administrator may cancel from `Received`, `Diagnosing`, or `InProgress`. Technician cannot cancel. Cancellation from `Ready`, `Completed`, or `Cancelled` is rejected.
2. Administrator may perform every Receptionist and Technician action, plus staff-account management, loyalty-tier changes, and RepairOrder corrections. No new operations or integrations are added.
3. Administrator may correct any RepairOrder field, including on `Completed` or `Cancelled` orders, if the result still satisfies required-field, relationship, uniqueness, pricing, and lifecycle rules. Terminal orders cannot be reopened. Approval, repair-check, payment, and pickup guards still apply.
4. Every successful change is audited (actor, time, record, field, old and new value) and keeps prior history. A rejected correction changes nothing.
5. The Administrator product role grants no authority over project Gate or Validation decisions.

## Test impact

| Requirements | What to test | Scenarios |
|---|---|---|
| REQ-F-002, REQ-NF-002 | Every role/action pair; Administrator can do Receptionist and Technician actions; Technician cancellation is denied. | BDD-011, 018, 019, 020 |
| REQ-F-014 | Both cancelling roles in all three source states; forbidden source states; guard denials repeated as Administrator. | BDD-004, 005, 006, 009, 010, 018, 019 |
| REQ-F-018, REQ-NF-003 | Valid and invalid corrections, including terminal records; full before/after audit history; no partial writes. | BDD-016; Administrator runs of BDD-004, 009, 010 |
