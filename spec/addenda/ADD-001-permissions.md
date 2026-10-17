# ADD-001: cancellation authority and Administrator permissions

- **Status:** Approved by the human product owner, Dmytro Shadura, on 2026-10-15.
- **Source baseline:** SRS 1.0.0 / SPEC-GATE-01, repository commit `31bf3317552f62adbafbebd32fa772b38c9c6a8e`.
- **Active specification:** SRS 1.0.1. Requirement IDs remain stable; this addendum records the approved change rather than rewriting the source baseline's history.
- **Decision ticket:** [FIX-24](https://linear.app/fixops/issue/FIX-24/confirm-staff-permissions).
- **Affected requirements:** REQ-F-002, REQ-F-014, REQ-F-018; cross-cutting verification of REQ-NF-002 and REQ-NF-003.

## Human decisions

The product owner stated that Receptionist and Administrator may cancel orders, and Administrator may do everything. In the follow-up clarification, the product owner explicitly chose full permissions with business rules and audit remaining mandatory.

## Approved policy

1. Receptionist and Administrator may cancel a RepairOrder from the cancellation source states already permitted by SRS: Received, Diagnosing and InProgress, subject to their existing conditions. Technician cannot cancel. Cancellation from Ready, Completed or Cancelled remains rejected.
2. Administrator may perform every action assigned to Receptionist or Technician, as well as staff-account management, manual loyalty-tier changes and RepairOrder corrections. Full permissions apply to actions within the approved release; this does not introduce new operations or integrations.
3. Administrator may correct any RepairOrder field, including on a Completed or Cancelled order, provided the resulting record satisfies the same required-field, relationship, uniqueness, pricing and lifecycle constraints. A correction is not a way to bypass a denied operation. Terminal status cannot be reopened, cancellation cannot be forced from Ready, and payment, pickup, repair-check and client-approval guards remain mandatory.
4. Every successful change retains the required audit evidence: actor, time, record, field, previous value and new value. Corrections preserve prior audit history. A rejected correction leaves the affected business record unchanged.
5. Administrator is a role in the product. It does not confer authority to make the human owner's project Gate or intent-Validation decisions.

## Acceptance and traceability impact

| Requirement / criterion | Required planning and test update | Scenario references | Delivery work |
|---|---|---|---|
| REQ-F-002 / AC-F-002; REQ-NF-002 / AC-NF-002 | Exercise every existing role/action pair; Administrator can execute Receptionist and Technician responsibilities when guards hold. Technician cancellation is denied without changing the order. | BDD-011, BDD-018, BDD-019, BDD-020 | FIX-27 through FIX-29; role checks in P2-P6; ACCESS-GATE FIX-48 |
| REQ-F-014 / AC-F-014 | Exercise both authorized cancellation roles at all three permitted source states, denied Technician cancellation, forbidden source states and unchanged transition guards. Repeat approval/check/payment/pickup guard denials as Administrator. | BDD-004, BDD-005, BDD-006, BDD-009, BDD-010, BDD-018, BDD-019 | FIX-36 through FIX-41; FINANCIAL-DATA-GATE FIX-49 |
| REQ-F-018 / AC-F-018; REQ-NF-003 / AC-NF-003 | Test all correction fields with valid/invalid values, including terminal records; preserve full before/after audit history. Reject guard bypass and inconsistent settlement changes without partial business-record writes. | BDD-016; Administrator guard checks in BDD-004/009/010 | FIX-42 through FIX-44; shared audit contract in FIX-27 through FIX-29; FINANCIAL-DATA-GATE FIX-49 |

SRS 1.0.1 contains 23 requirement IDs, 23 acceptance criteria and 20 BDD scenarios. The traceability matrix records these additional scenario and addendum links. Executable tests and product implementation remain future work; approval of this requirement clarification is not PLAN-GATE GO.
