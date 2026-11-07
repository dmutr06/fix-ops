# FINANCIAL-DATA-GATE

**Decision:** GO · **Ticket:** [FIX-49](https://linear.app/fixops/issue/FIX-49) · **Scope:** REQ-F-010–016, REQ-F-018, REQ-NF-001, REQ-NF-003, REQ-NF-004 · **Evidence:** [P4](verification_p4.md), [P5](verification_p5.md), [P6](verification_p6.md), [P7](verification_p7.md), [SRS fixes](verification_srs_fixes.md)

Pricing, lifecycle, audit, performance and recovery are verified. Human validation may start.

## Evidence

| Check | Result |
|---|---|
| Lifecycle transitions and guards | P4 GO; all 36 lifecycle pairs covered in P5 tests |
| Cancellation charges, rounding, current tier, exact payment | P5 GO; half-cent rounding and tier concurrency verified |
| Administrator corrections keep rules and audit, including terminal records | P6 GO; terminal settlement preserved (FIX-56) |
| Audit atomicity | Audit rows written in the same transaction; rollback and denied-action tests pass |
| Performance (AC-NF-001) | API p95 39.7–103.9 ms with 10 concurrent staff and 10,000 orders; no errors |
| Restore (AC-NF-004) | Real records and audit rows matched after restore; failed backup and restore checks pass |
| Full suite after fixes | 218 tests, fmt and Clippy clean |

## Accepted limitations

| Item | Disposition |
|---|---|
| Browser-visible timing for AC-NF-001 | API timing only; no frontend exists |
| Daily backup schedule | Documented in [recovery](../docs/recovery.md); installed per deployment |
| 24-hour backup observation | Operational work after deployment |
