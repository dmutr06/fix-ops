# TEST-GATE P8: Web API changes

**Decision:** GO (delegated; human review follows) · **Ticket:** [FIX-62](https://linear.app/fixops/issue/FIX-62) · **Scope:** REQ-F-001, REQ-F-002, REQ-F-022, REQ-F-023, REQ-NF-002 · **Baseline:** [SRS 1.1.0](../spec/srs.md), [ADD-004](../spec/addenda/ADD-004-web-interface.md), [`openapi.yaml`](../spec/openapi.yaml) 1.1.0

Tests come from ADD-004 and the contract, before any implementation.

| Requirement | Source | Tests |
|---|---|---|
| REQ-F-001, REQ-F-022 | AC-F-022, BDD-022, `cookieAuth` | `bdd_022_sign_in_sets_an_http_only_cookie_that_sign_out_ends`, `current_session_and_sign_out_need_a_session` |
| REQ-F-002, REQ-NF-002 | AC-F-002, BDD-024, `x-roles` | `bdd_024_every_role_lists_staff_but_only_admin_manages`, `role_matrix_matches_x_roles`, `every_protected_operation_has_an_action`, domain `every_role_reads_records_and_audit`; staff tests now deny account changes instead of the list |
| REQ-F-023 | AC-F-023, BDD-023 | `bdd_023_each_role_sees_its_work_queue`, `every_list_pages_and_counts`, `queue_filters_reject_invalid_values` |
| Contract | 28 operations | `every_contract_operation_is_routed`, `every_operation_traces_to_requirements` |

Baseline: new tests fail for the expected reasons: no `/sessions/current` route, no `Set-Cookie`, no `X-Total-Count`, unknown filters rejected, role matrix differs from `x-roles`; domain tests do not compile without the two session actions. `queue_filters_reject_invalid_values` already passes, since unknown query parameters were rejected before.
