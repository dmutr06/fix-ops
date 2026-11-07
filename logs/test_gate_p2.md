# TEST-GATE P2: Client, device and order intake

**Decision:** GO · **Tickets:** [FIX-30](https://linear.app/fixops/issue/FIX-30) (tests), [FIX-31](https://linear.app/fixops/issue/FIX-31) (feature) · **Scope:** REQ-F-003, REQ-F-004, REQ-F-005, REQ-F-017, with REQ-F-001, REQ-F-002, REQ-NF-002, REQ-NF-003 for the new endpoints · **Baseline:** [SRS 1.0.2](../spec/srs.md), [ADD-001](../spec/addenda/ADD-001-permissions.md), [ADD-002](../spec/addenda/ADD-002-api-contract.md), [`openapi.yaml`](../spec/openapi.yaml), [architecture](../docs/architecture.md)

Tests were written from the SRS, the addenda, the glossary and the contract before any implementation. Expected values come from those documents, never from code output.

## Coverage

| Requirement | Source | Tests |
|---|---|---|
| REQ-F-003 | AC-F-003, BDD-001, BDD-002, `ClientCreateRequest`, `ClientUpdateRequest`, `listClients` | `bdd_001_receptionist_links_client_and_device_to_a_received_order`, `client_requires_name_and_phone`, `client_email_is_optional_and_nullable`, `client_values_are_trimmed`, `invalid_client_email_is_rejected`, `client_create_rejects_unknown_fields_and_malformed_json`, `client_update_changes_only_the_given_fields`, `client_update_rejects_invalid_values`, `client_search_matches_name_or_phone_ignoring_case`, `client_search_rejects_invalid_parameters`; domain: `name_and_phone_are_trimmed_and_required`, `email_must_be_an_address`, `update_records_only_changed_fields`, `update_can_clear_and_set_the_email` |
| REQ-F-004 | AC-F-004, BDD-001, BDD-002, `DeviceCreateRequest`, `DeviceUpdateRequest`, `listDevices` | `bdd_002_device_without_serial_number_is_rejected`, `device_requires_type_serial_number_and_symptoms`, `device_of_unknown_client_is_not_found`, `device_update_changes_only_the_given_fields`, `device_update_rejects_invalid_values`, `device_list_filters_by_client_and_serial_number`; domain: `device_type_parses_its_three_names_only`, `serial_number_and_symptoms_are_trimmed_and_required`, `update_records_only_changed_fields` (device) |
| REQ-F-005 | AC-F-005, BDD-001, BDD-002, `createRepairOrder` responses (`404`, `409`), `RepairOrder`, `listRepairOrders` | `bdd_001_receptionist_links_client_and_device_to_a_received_order`, `bdd_001_administrator_can_run_intake`, `bdd_002_duplicate_ticket_number_is_rejected`, `order_requires_a_ticket_number_and_every_link`, `order_with_an_unknown_link_is_not_found`, `order_for_a_device_of_another_client_conflicts`, `order_assignee_must_be_a_technician`, `order_list_filters_by_status_client_and_ticket`; domain: `status_parses_the_six_contract_names_only`, `ticket_number_is_trimmed_and_required`, `intake_links_the_client_its_device_and_a_technician`, `intake_rejects_a_device_of_another_client`, `intake_assigns_only_a_technician`, `a_received_order_has_nothing_recorded`, `a_received_order_owes_nothing` |
| REQ-F-017 | AC-F-017, BDD-011, BDD-015 (precondition), `setClientLoyaltyTier` | `bdd_011_technician_cannot_change_a_loyalty_tier`, `bdd_015_administrator_changes_the_tier_and_it_is_audited`, `invalid_loyalty_tier_is_rejected`; domain: `loyalty_tier_parses_its_three_names_only`, `tier_change_is_one_audited_field` |
| REQ-F-001 | AC-F-001, BDD-017 | `bdd_017_visitor_gets_no_client_device_or_order` (no token and unknown token on all 13 new operations; body is only `code` and `message`; no record content; nothing changes) |
| REQ-F-002, REQ-NF-002 | AC-F-002, ADD-001 §2, ADD-002 §1, `x-roles` | `every_role_reads_and_searches_clients_devices_and_orders`, `receptionist_and_administrator_maintain_clients_devices_and_orders`, `technician_cannot_maintain_clients_devices_or_orders`, `denied_roles_get_403_whatever_the_request`, `bdd_011_technician_cannot_change_a_loyalty_tier`; the matrix itself stays checked by `tests/permissions.rs` |
| REQ-NF-003 | AC-NF-003, [audit contract](../docs/architecture.md#audit-contract) | `creation_is_audited_for_clients_devices_and_orders`, `updates_are_audited_with_old_and_new_values`, `bdd_015_administrator_changes_the_tier_and_it_is_audited`; domain: `creation_audits_every_set_field_and_the_starting_tier`, `creation_audits_every_field`, `creation_audits_ticket_links_and_status` |
| Money (SRS §3.2, ADD-002 §4) | `*_cents` integers, `minimum: 0` | `assert_received_order` (integer `0` in all four cents fields, used by every order test); domain: `negative_amounts_are_rejected_naming_the_field`, `zero_and_positive_amounts_are_kept_exactly`, `renders_as_whole_cents` |
| ADD-002 §5 | Path parameters `minimum: 1`, unified `ErrorResponse` | `record_ids_must_be_positive_integers`, `unknown_records_are_not_found` |

Negative and boundary cases: missing, empty, whitespace-only and `null` name, phone, serial number, symptoms and ticket number; invalid `device_type` (`Phone`, lowercase `laptop`); seven malformed emails, with `null` email allowed on create and update; unknown fields (`loyalty_tier` and `id` on client create, `client_id` on device update, `status` and `used_parts_cents` on order create); malformed JSON, arrays and strings as bodies; ids `0`, `-1`, `abc`, `1.5` in paths, and `0`, `-5`, `"1"`, `1.5` in bodies; unknown client, device and technician; a device of another client; a Receptionist and an Administrator as assignee; a duplicate ticket number, also with surrounding spaces; empty `q` and `ticket_number`, unknown query parameters, lowercase and unknown `status`; `%` and `_` in a search term match literally; denied roles with invalid bodies, invalid ids and unknown ids (403 first); every denied or rejected request followed by a read-back of clients, devices, orders and the audit log.

## Decisions taken while writing the tests

| Question | Decision | Basis |
|---|---|---|
| May an Administrator be the assigned Technician? | No. Only an account whose role is `Technician` can be assigned; an Administrator or Receptionist assignee gets `409 CONFLICT`. | REQ-F-005 ("assigned Technician"), the glossary (RepairOrder "an assigned Technician"; Technician is a role) and `assigned_technician_id`. ADD-001 §2 lets an Administrator *perform* Technician actions; it does not make the account a Technician. Assigning an Administrator would add a rule the spec does not state. |
| Unknown client, device or technician on `createRepairOrder` | `404 NOT_FOUND`, `details.field` names the link | The operation documents `404`; ADD-002 §5: `404` is an unknown record. |
| Device of another client, assignee not a Technician | `409 CONFLICT`, `details.field` names the link | The operation documents `409` for "Duplicate ticket number or invalid device/technician relationship". `GUARD_VIOLATION` is for lifecycle and settlement guards (architecture § Errors), so `CONFLICT` is used. |
| Duplicate ticket number | `409 CONFLICT`, `details.ticket_number` | Contract example `duplicateTicket`. Ticket numbers are trimmed, so a padded duplicate is the same ticket. |
| Unknown client on `createDevice` | `404 NOT_FOUND`, `details.field = client_id` | The operation documents `404`. |
| Order audit on creation | `ticket_number`, `client_id`, `device_id`, `assigned_technician_id`, `status` | Architecture: creation logs each set field. Zero charges and `repair_checked = false` are defaults, not entered values. |
| `labor_discount_cents`, `settlement_cents` | Derived by `pricing::current_settlement`; `0` for a fresh `Received` order | SRS §3.2 (cancellation before findings is $0), architecture § Settlement. Open-order pricing beyond `Received` is P5. |

## Baseline run (red)

`cargo test --workspace --no-fail-fast` on the test commit:

| Target | Passed | Failed |
|---|---|---|
| `tests/intake.rs` | 0 | 35 |
| `fixops-domain` unit tests | 16 | 22 |
| Existing suites (staff_access, permissions, logging, contract, platform, app, infra, api) | 58 | 0 |

- All 35 HTTP failures are `unexpected status`, `404 NOT_FOUND "Requested record does not exist"` from the router fallback at the first request to a new endpoint: the routes do not exist yet. Setup through existing endpoints (bootstrap, staff, sessions) succeeds in every test.
- All 22 domain failures panic with `not yet implemented` in the stubbed constructors and rules (`Cents::new`, `ClientName::parse`, `Email::parse`, `LoyaltyTier::parse`, `DeviceType::parse`, `TicketNumber::parse`, `OrderStatus::parse`, `NewRepairOrder::intake`, the change builders and `current_settlement`).
- They fail because the behavior is missing, not because a test is wrong. The domain tests compile against the final signatures.

## Checks

- Every test names its requirement or scenario and asserts status, `ErrorResponse.code` and body fields from the contract; response bodies are compared with the exact key sets of `Client`, `Device` and `RepairOrder`.
- Denied and rejected actions are verified by reading clients, devices, orders and the audit log back.
- The tests use only the HTTP API and the domain crate, not table names, so they do not depend on the schema.
- No test data contains real personal data or credentials.

## Open questions (not tested, spec is silent)

- Whether a disabled Technician account may be assigned. The spec does not say; it is allowed as long as the role is Technician.
- Whether ticket numbers are case-sensitive. They are compared exactly after trimming.

**Next:** implement FIX-31 until these tests pass without changing them.
