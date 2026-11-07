# TEST-GATE P1: Staff access and accounts

**Decision:** GO · **Tickets:** [FIX-27](https://linear.app/fixops/issue/FIX-27) (tests), [FIX-28](https://linear.app/fixops/issue/FIX-28) (feature) · **Scope:** REQ-F-001, REQ-F-002, REQ-F-019, REQ-NF-002, REQ-NF-003 · **Baseline:** [SRS 1.0.2](../spec/srs.md), [ADD-001](../spec/addenda/ADD-001-permissions.md), [ADD-002](../spec/addenda/ADD-002-api-contract.md), [`openapi.yaml`](../spec/openapi.yaml)

Tests were written from the SRS, the addenda and the contract before any implementation. Expected values come from those documents, never from code output.

## Coverage

| Requirement | Source | Tests |
|---|---|---|
| REQ-F-001 | AC-F-001, BDD-017, `bearerAuth` | `bdd_017_request_without_token_is_denied`, `bdd_017_unknown_token_is_denied`, `bdd_017_non_bearer_authorization_is_denied`, `bdd_017_expired_session_is_denied`, `sign_in_returns_token_and_account`, `every_sign_in_issues_a_distinct_token`, `wrong_password_and_unknown_user_are_indistinguishable`, `sign_in_rejects_malformed_body`, `only_sign_in_is_public` |
| REQ-F-002, REQ-NF-002 | AC-F-002, AC-NF-002, ADD-001 §2, ADD-002 §2, `x-roles` | `role_matrix_matches_x_roles`, `every_protected_operation_has_an_action`, `receptionist_and_technician_cannot_manage_staff`, `role_change_applies_to_the_existing_session`, `every_role_reads_the_audit_log`; domain: `administrator_inherits_every_staff_action`, `technician_cannot_cancel_or_take_front_desk_actions`, `receptionist_cannot_do_bench_work`, `only_administrator_manages_staff_tiers_and_corrections`, `every_role_reads_records_and_audit`, `require_reports_the_denied_action` |
| REQ-F-019 | AC-F-019, `StaffCreateRequest`, `StaffUpdateRequest` | `administrator_creates_an_account_in_each_role`, `created_administrator_can_manage_staff`, `duplicate_username_conflicts`, `invalid_staff_account_is_rejected`, `invalid_staff_update_is_rejected`, `update_of_unknown_staff_account_is_not_found`, `staff_id_must_be_a_positive_integer`, `disabled_account_cannot_sign_in_and_loses_its_session`, `re_enabled_account_can_sign_in_again`, `disabling_one_account_keeps_other_sessions`, `username_is_trimmed`, `bootstrap_runs_only_while_no_staff_exist`; domain: `username_is_trimmed_and_required`, `password_needs_eight_characters` |
| REQ-NF-003 | AC-NF-003, audit contract in [architecture](../docs/architecture.md#audit-contract) | `staff_creation_is_audited_without_the_password`, `staff_updates_are_audited_in_order`, `audit_log_filters_by_record`, `audit_log_rejects_invalid_filters`, `no_password_or_hash_is_ever_returned`; domain: `creation_audits_every_field_and_redacts_the_password`, `update_records_only_changed_fields`, `update_keeps_omitted_fields`, `password_debug_hides_the_secret` |

Negative and boundary cases: missing, unknown, non-bearer and expired tokens; wrong password vs unknown user (same response); disabled and re-enabled accounts; 7- vs 8-character passwords; empty and whitespace usernames; unknown JSON fields; ids `0` and non-numeric; every denied role on every `/staff` operation, with a read-back confirming nothing changed.

## Baseline run (red)

`cargo test --workspace --no-fail-fast` on the test commit:

| Target | Passed | Failed |
|---|---|---|
| `tests/staff_access.rs` | 0 | 28 |
| `tests/permissions.rs` | 1 | 2 |
| `fixops-domain` unit tests | 3 | 13 |
| Existing suites (contract, platform, app, api) | 8 | 0 |

All 43 failures panic with `not yet implemented` in stubbed domain functions (`Role::can`, `Action::roles`, `Username::parse`, `Password::parse`, audit change builders). They fail because the behavior is missing, not because a test is wrong. The tests compile against the final signatures.

## Checks

- Every test names its requirement or scenario and asserts status, `ErrorResponse.code` and body fields from the contract.
- The permission matrix is checked against `x-roles` parsed from `openapi.yaml`, so it cannot drift from the contract.
- Denied actions are verified by reading the record and the audit log back.
- No test data contains real credentials.

**Next:** implement FIX-28 until these tests pass without changing them.
