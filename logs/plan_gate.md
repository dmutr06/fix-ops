# FixOps PLAN-GATE evidence and decision record

- **Recorded:** 2026-10-15, Europe/Kyiv.
- **Planning source:** Approved SRS 1.0.0 / SPEC-GATE-01 at `31bf3317552f62adbafbebd32fa772b38c9c6a8e`.
- **Tracker:** [FixOps](https://linear.app/fixops/project/fixops-965d39cb6a8a), Personal workspace, FIX team.
- **Decision issue:** [FIX-26 / PLAN-GATE](https://linear.app/fixops/issue/FIX-26/plan-gate-approve-the-plan).
- **Human decision authority:** Dmytro Shadura.
- **PLAN-GATE decision:** **GO** — Dmytro approved the plan in this chat and asked the agent to enter the approval.
- **Approval scope:** prepared specification review, ADD-001 and delivery-planning package. No product verification, intent Validation or release approval is asserted.

## Authorization and human corrections

The human explicitly approved implementation of the delivery-planning proposal: create the Linear project and 29 tickets, configure dependency controls, and publish project artifacts. That authorization covers preparation of this planning package, not systematic implementation of the product or release approval.

The agent proposed SRS-based decomposition, workflow, risk controls and a provisional calendar. During planning, the human made these decisions:

| Topic | Agent proposal / initial uncertainty | Human decision reflected in the result |
|---|---|---|
| Current delivery | Planning artifacts versus implementing the full web system | Planning and Linear only; product implementation and a separate report are excluded from this change. |
| Tracker structure | FixOps exists as a team, with no project | Create a FixOps project inside the existing Personal FixOps team. |
| Language | Ukrainian was the suggested default | Use English for tickets and project documentation. |
| Calendar | Four working weeks was the suggested default | Target two working weeks, 19-30 October 2026; retain dependencies and risk disclosure. |
| Human capacity | Separate human review allocation | One hour per weekday for review, clarification, Validation and Gate decisions. |
| Existing wording | Keep or edit older project documents | Remove educational framing from current documents; preserve Git history, requirement IDs, business rules and historical decisions. |
| Tooling | Connector plus possible workflow-settings UI inspection | Use only the Linear connector; reuse the existing type labels and workflow states instead of requiring browser configuration. |
| Execution | Independent analysis and preparation | Use parallel agents with disjoint file ownership, then integrate and verify the result. |

On 2026-10-15, Dmytro explicitly authorized Receptionist and Administrator to cancel orders and granted Administrator all existing staff permissions. In the follow-up clarification, Dmytro confirmed that business rules and audit remain mandatory. [ADD-001](../spec/addenda/ADD-001-permissions.md) records this approval and SRS 1.0.1 incorporates it. [FIX-24](https://linear.app/fixops/issue/FIX-24/confirm-staff-permissions) is accepted on that human decision; no human PLAN-GATE GO was expressed. Native Addendum dependencies remain as satisfied history, while all other Test and Gate prerequisites remain in force.

## Prepared evidence

- [Delivery plan](../docs/delivery_plan.md): seven packages, 29 linked issues, five milestone targets, Gantt and a manually justified calendar-critical chain.
- [Workflow](../docs/workflow.md): eight existing type labels, role-annotated task lifecycle, exact issue dependency tables and human-only acceptance responsibilities.
- [Risk register](../docs/risk_register.md): 14 concrete risks, all six AI risk categories, owners, detection triggers and linked controls.
- [Traceability](../spec/traceability.md): all 23 requirements linked to Test/Feature/Verification tickets and human control issues; ADD-001 adds BDD-018/019/020, bringing active scenario coverage to 20. Automated-test and code columns remain empty.
- Human schedule: 490 minutes across ten weekdays, no more than 60 minutes on any date. Technical budgets total 63 agent-hours and have not been measured against a selected web/storage stack.

## Independent agent contribution

One agent independently mapped the 23 requirements and BDD coverage, then owned the workflow document. A second independently checked the planning brief and prototype gaps, then owned the risk register. Before execution, the second agent identified concrete plan corrections: explicit downstream Gate blockers; Addendum blocking tests as well as Features; an unambiguous Review representation; allocated human time; client-visible performance timing; an actual restoration exercise; and an early shared audit contract. These corrections are incorporated into tickets and documents. The coordinating agent owns Linear writes, calendar, traceability, integration and final checks.

## Planning-package verification

Initial planning integration checks on 2026-10-15, before ADD-001:

- Read back every issue through the Linear connector: exactly 29 issues in the intended project/team, all assigned to Dmytro, with matching types, due dates and milestones. All 96 native blocking edges match the plan; the graph has no cycles and predecessor due dates do not exceed successor start dates.
- Asserted Test-before-Feature, PLAN/ACCESS/FINANCIAL-DATA/RELEASE dependencies and clarification blockers; no issue is Done and no human Gate GO is recorded.
- Checked all 23 traceability rows, requirement/AC references and 17 BDD IDs. Compared SRS sections 2-7 with the source baseline byte-for-byte: approved roles, business rules, requirements and scenarios are unchanged. Automated-test and code links remain empty.
- Checked 490 human minutes, daily maximum 60 minutes, 63 technical hours, and the two-lane aggregate capacity. Independent agent review confirmed the calculations and dependency ordering; its missing workflow milestone finding was corrected in the Gantt and milestone table.
- Checked 14 risk entries, all six AI categories, valid local document links, actual tracker links and removal of educational framing.
- Ran `git diff --check` successfully and `cargo test --offline`: all three existing integration tests pass. No product source, test or dependency file was modified.

The later human-approved ADD-001 amends role permissions and correction constraints in SRS 1.0.1; the original source baseline and initial check results above remain historical evidence. FIX-24 completion records requirement approval, not product conformance or a Gate GO.

After ADD-001, targeted reference and policy assertions passed for all 23 requirement/AC rows and 20 BDD scenarios. Independent agent review confirmed permission, terminal-state, guard and audit consistency; its stale active-version reference in risk R-09 was corrected. Readback of all 29 Linear issues confirmed active SRS 1.0.1 references, matching scenario links, all 96 dependency edges preserved, FIX-24 Done, and no other state changes or Gate approvals. `git diff --check` passed. Product source, executable tests and dependencies were not changed; the prototype test suite was not rerun for this documentation-only amendment.

These checks establish planning consistency only; they cannot demonstrate product conformance or substitute for human acceptance. The initial web/storage architecture is not selected, future acceptance tests have not been created, and the 24-hour backup/restore exercise has not been performed by this planning change. A missed review or optimistic engineering budget requires reforecasting rather than bypassing controls.

## Human decision procedure

Before deciding PLAN-GATE, review FIX-23 specification review, FIX-24 clarification and FIX-25 planning documentation. Confirm full requirement coverage, actual dependency direction without cycles, tests before Features, appropriate high-consequence controls, feasible human review slots and concrete risk mitigations. Review the already approved and versioned ADD-001 when accepting dependent tests. Re-estimate the target after the initial platform design review if the prototype gaps exceed the budgets.

Dmytro decides each Gate. He may add a comment himself or ask the agent to enter his decision:

- **GO:** accepted prerequisites and risk controls permit the dependent work to start.
- **REVISE:** return named gaps to requirements/planning; implementation remains blocked.
- **BLOCK:** record the missing prerequisite, owner and condition for reconsideration; implementation remains blocked.

The approval below is based on the human's explicit instruction. ACCESS-GATE, FINANCIAL-DATA-GATE, intent Validation and RELEASE-GATE remain future human decisions requiring their own actual evidence.


## PLAN-GATE: GO

Dmytro read the planning materials, agreed with them and asked the agent to enter the approvals in Linear.

- **Accepted:** specification review (FIX-23), permission Addendum (FIX-24) and planning documents (FIX-25).
- **Decision:** GO for PLAN-GATE (FIX-26). These four tasks are Done.
- **Checks:** 23 requirements, 20 linked scenarios, 29 tasks, five milestones, 96 acyclic dependency links, review time within one hour/day and 14 risks.
- **Next:** prepare the staff-access tests in FIX-27. Features still need their own accepted tests and later Gates.
- **Still open:** ACCESS-GATE, FINANCIAL-DATA-GATE, staff-usage Validation and RELEASE-GATE need actual implementation and verification results.
- **Schedule:** the two-week target remains a forecast. Update it if platform work or review takes longer than planned.

The approval is Dmytro's decision; the agent entered it at his request. No extra recorder or timestamp fields are used. The Linear comments and change history retain the approval trail.


## Shorter Linear tickets

At Dmytro's request, all 29 ticket titles and descriptions were shortened to a goal, checklist and requirement references. Ownership/effort and dependency/handoff sections were removed from descriptions; the existing Linear fields, all 96 blockers, approvals and repository estimates were preserved. Readback confirmed matching owners, dates, types, milestones and statuses.
