# PLAN-GATE

**Decision:** GO · **Ticket:** [FIX-26](https://linear.app/fixops/issue/FIX-26) · **Scope:** SRS 1.0.1 (1.0.0 + [ADD-001](../spec/addenda/ADD-001-permissions.md))

Accepted: specification review (FIX-23), permission addendum (FIX-24), and planning documents (FIX-25). This approves the plan only, not any product verification, validation, or release.

## Planning choices

| Topic | Decision |
|---|---|
| Deliverable | Planning documents and Linear tickets only; no product code |
| Tracker | New FixOps project in the existing FIX team |
| Language | English |
| Calendar | Two working weeks, 19–30 October 2026 |
| Review capacity | One hour per weekday for all review, validation, and gate work |
| Execution | Parallel agents with separate file ownership, then integration |
| Permissions | Receptionist and Administrator can cancel; Administrator can do everything, with business rules and audit still enforced ([ADD-001](../spec/addenda/ADD-001-permissions.md)) |

## Checks

- 29 Linear issues in the right project, each with an owner, type, dates, and milestone. 96 blocking links, no cycles, and every Test comes before its Feature.
- 23 traceability rows, all with valid AC and BDD references (20 scenarios).
- Human review: 490 minutes total, max 60 per day. Technical: 63 agent-hours, not yet measured against a real stack.
- 14 risks covering all six AI risk categories.
- `git diff --check` passes; `cargo test --offline` passes 3/3 prototype tests. No product code changed.

An independent agent review led to these plan corrections: explicit gate blockers, the addendum blocking tests, time allocated for human review, performance measured to the visible result, a real restore exercise, and an early shared audit contract.

## Still open

ACCESS-GATE, FINANCIAL-DATA-GATE, human validation, and RELEASE-GATE need real implementation evidence. The platform architecture is not chosen yet, and the schedule will be re-estimated after the P1 platform review.

**Next:** staff-access tests (FIX-27).
