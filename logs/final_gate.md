# FINAL-GATE

**Decision:** GO · **Ticket:** [FIX-59](https://linear.app/fixops/issue/FIX-59) · **Scope:** [SRS 1.0.3](../spec/srs.md) with ADD-001, ADD-002 and ADD-003 · **Version:** `v1.0.0`

The documented service meets the [concept](../spec/concept.md) for this cycle: staff record clients and devices, take an order from intake through diagnosis, estimate, approval and repair to payment, pickup and settlement, with the concept's diagnostic fee and labor-only loyalty discount. The missing staff web interface is the next cycle.

## Cycle

| Stage | Record |
|---|---|
| Concept and discovery | [SKED dialogue](sked_dialogue.md), [GIT-GATE](git_gate.md) |
| Specification | [SPEC-GATE](spec_gate.md): SRS 1.0.0 baseline |
| Plan, tickets, risks | [PLAN-GATE](plan_gate.md), [delivery plan](../docs/delivery_plan.md), [risk register](../docs/risk_register.md) |
| API contract | [API-GATE](api_gate.md): `openapi.yaml` checked with a Prism mock |
| Implementation | P1–P7, each TEST-GATE → feature → verification; [ACCESS-GATE](access_gate.md), [FINANCIAL-DATA-GATE](financial_data_gate.md) |
| Change control | [SPEC-ADDENDUM-GATE](spec_addendum_gate.md), [ADD-003 verification](verification_add003.md), [review fixes](verification_srs_fixes.md) |
| Validation and release | [Staff workflow](validation_staff_workflow.md), [RELEASE-GATE](release_gate.md) |
| Documentation | [Final verification](final_verification.md) |

## Addenda

| Addendum | Change |
|---|---|
| [ADD-001](../spec/addenda/ADD-001-permissions.md) | Cancellation roles; Administrator inherits staff actions without bypassing rules |
| [ADD-002](../spec/addenda/ADD-002-api-contract.md) | REST read permissions, error model and contract conventions |
| [ADD-003](../spec/addenda/ADD-003-last-enabled-administrator.md) | At least one enabled Administrator is kept |

## Human decisions

- Scope: the full order cycle through settlement; no portal, notifications, inventory, payment provider or analytics.
- Cancellation in three stages ($0, $30 after saved findings, incurred costs after repair) instead of the prototype's flat $30.
- The client's tier at final settlement applies; tiers change only by hand.
- Administrator is not a bypass: one role matrix, the same guards and audit.
- API first; the staff web interface is planned separately.
- A correction of a terminal order's pricing inputs re-prices it with the client's current tier.

## Validation

The service was checked against the concept, not only the tests: every concept entity, state, fee and discount rule is implemented and traced in [`traceability.md`](../spec/traceability.md). The [user guide](../docs/user_guide.md) was walked through against a running service on a fresh database.

Not met: the staff **web interface** of SRS §2 and browser timing for REQ-NF-001. This is a missing client of the same API rather than a wrong behavior, so it starts the next SDD cycle (a new concept or addendum for the staff web app) instead of reopening this one. Operational items stay as at RELEASE-GATE: the installed daily backup schedule and a 24-hour backup observation.
