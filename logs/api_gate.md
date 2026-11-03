# API-GATE

**Decision:** GO · **Recommendation:** GO · **Scope:** [SRS 1.0.2](../spec/srs.md) ([ADD-001](../spec/addenda/ADD-001-permissions.md), [ADD-002](../spec/addenda/ADD-002-api-contract.md)) · **Contract:** [`spec/openapi.yaml`](../spec/openapi.yaml)

Per [`docs/workflow.md`](../docs/workflow.md#tickets), the agent prepares evidence and a recommendation; the final `GO` / `REVISE` / `BLOCK` decision is reserved for human review. No production server code was added to `src/`.

## Tooling and commands

| Tool | Pinned version | Command | Result |
|---|---|---|---|
| Node.js (`fnm`) | `v24.22.0` | `node --version` | Runtime for Redocly and Prism CLIs |
| Redocly CLI | `2.72.1` | `npx --yes @redocly/cli@2.72.1 lint spec/openapi.yaml` | `0 errors, 0 warnings` (46 ms) |
| Stoplight Prism CLI | `5.16.0` | `npx --yes @stoplight/prism-cli@5.16.0 mock spec/openapi.yaml -h 127.0.0.1 -p 4010` | Serves all 26 operations over HTTP |
| Rust HTTP mock suite | `rustc 1.99.1` | `cargo test --offline --test prism` | `6/6` suites passed (`39` HTTP checks, 0.08 s) |
| Supplementary offline checks | `openapiv3 2.2.0` | `cargo test --offline` | `3/3` unit (`src/order.rs`) + `3/3` spec-to-SRS checks (`tests/contract.rs`) passed |

## Contract review and reconciliation findings

1. **SRS alignment (`REQ-F-013`, `BDD-010`, `BDD-012`):** Decoupled `PUT /api/v1/orders/{orderId}/pickup` (`recordPickup`) from `POST /api/v1/orders/{orderId}/complete` (`completeRepairOrder`) so payment and pickup can be recorded in either order before requesting `Completed`.
2. **Search and read coverage (`REQ-F-003`–`005`, `REQ-NF-001`):** Added `GET /api/v1/clients`, `GET /api/v1/devices`, and `GET /api/v1/orders` and documented staff read/search rules in [`ADD-002`](../spec/addenda/ADD-002-api-contract.md).
3. **Unapproved field removal (`REQ-F-018`):** Removed the invented `reason` field from `RepairOrderCorrectionRequest` so payloads strictly match [`srs.md`](../spec/srs.md) and [`glossary.md`](../spec/glossary.md); split `ClientCreateRequest` (`POST`) from `ClientUpdateRequest` (`PATCH`).
4. **OpenAPI 3.0.3 schema fix:** Redocly CLI initially reported 8 `nullable-type-sibling` errors where `nullable: true` was paired with `allOf` `$ref` properties. Replaced with explicit `NullableEstimate`, `NullableClientDecision`, `NullablePaymentRecord`, and `NullablePickupRecord` object schemas (`type: object`, `nullable: true`), resolving all errors and example warnings.
5. **Re-validation pass (`REQ-F-002`, `REQ-F-016`):** Independent re-run confirmed Redocly `0 errors`, Spectral `spectral:oas` `0 errors` (27 style warnings for missing `operation.description` / `info.contact`, non-blocking), and no Prism response violations. Closed coverage gaps: added `readyOrderHalfCentSilver` example and an HTTP check for `BDD-014` (half-cent rounding); added Administrator `PUT /diagnosis` check for `BDD-020`; labelled Receptionist/Administrator cancellations as `BDD-013`/`BDD-018`. `RepairOrder` now lists `diagnostic_findings`, `estimate`, `client_decision`, `payment`, `pickup` as `required` (always present, `null` when absent), matching every example. Corrected the HTTP check count (previously recorded as 38; actual was 37, now 39).

## Checklist

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Every API operation is linked to `REQ-ID` | PASS | All 26 operations in [`openapi.yaml`](../spec/openapi.yaml) (`x-requirements`) match [`traceability.md`](../spec/traceability.md) (`REQ-F-001`–`019`, `REQ-NF-001`–`003`; `REQ-NF-004` is an infrastructure backup job). |
| 2 | Resources and terms match the domain glossary | PASS | Schemas use exact [`glossary.md`](../spec/glossary.md) terms: `Client`, `Device`, `RepairOrder`, `StaffAccount`, `Estimate`, `ClientDecision`, `PaymentRecord`, `PickupRecord`, `AuditEntry`, `LoyaltyTier`, `OrderStatus`. |
| 3 | URIs and HTTP methods are used consistently | PASS | `/api/v1/{sessions,staff,clients,devices,orders,audit-logs}`: `POST` creates records or triggers state transitions (`complete`, `cancel`), `GET` reads/lists, `PUT` sets sub-resources, `PATCH` applies partial updates/corrections. |
| 4 | Request and response schemas are defined | PASS | 34 component schemas with `additionalProperties: false`, strict enums, and integer USD cents (`*_cents >= 0`). |
| 5 | HTTP status codes and unified error model | PASS | `200`, `201`, `400`, `401`, `403`, `404`, `409`; all errors return `ErrorResponse` (`code`, `message`, optional `details`). |
| 6 | Positive and negative scenarios are covered | PASS | 6 Rust HTTP test modules in [`tests/prism/`](../tests/prism/mod.rs) cover `P1`–`P6` positive flows and `400`/`401`/`403`/`404`/`409` negative flows. |
| 7 | No internal implementation details leaked | PASS | Contract exposes domain resources and integer cents rather than prototype `f64` fields or storage/ORM internals. |
| 8 | OpenAPI specification passes automatic validation | PASS | `@redocly/cli@2.72.1 lint spec/openapi.yaml` passes with 0 errors and 0 warnings; supplementary [`tests/contract.rs`](../tests/contract.rs) passes. |
| 9 | Mock API runs directly from `spec/openapi.yaml` | PASS | `@stoplight/prism-cli@5.16.0` serves `spec/openapi.yaml` over `http://127.0.0.1:4010` with zero server code in `src/`. |
| 10 | Positive and negative scenarios pass on Mock API | PASS | `cargo test --offline --test prism` checks status codes, Prism validation results, and key response fields for all 39 HTTP requests; response shapes are covered by Redocly example validation. |

## HTTP mock scenario results and limitations

 Scenarios in [`tests/prism/`](../tests/prism/mod.rs) distinguish three mock behaviors:

- **Default (18 requests):** Valid authenticated requests without `Prefer` headers (`POST /sessions`, `POST/PATCH/GET /staff`, `POST/GET /clients`, `PUT /loyalty-tier`, `POST /devices`, `POST /orders`, `PUT /diagnosis` (Technician and Administrator, `BDD-020`), `PUT /estimate`, `PUT /decision`, `PUT /payment`, `PUT /pickup`, `POST /complete`, `PATCH /orders/{id}`, `GET /audit-logs`). Prism validates headers/params/bodies (`no sl-violations`) and returns contract-compliant `200`/`201` payloads.
- **`.invalid()` (5 requests):** Requests intentionally omitting `Authorization` (`GET /orders/100` → `401`) or violating parameter/body schemas (`POST /devices` empty `serial_number` → `400`; `GET /orders/0` invalid `minimum: 1` → `400`; `PUT /decision` missing `decided_at` → `400`; `PATCH /orders/100` empty `{}` violating `minProperties: 1` → `400`). Prism rejects them automatically (`sl-violations` header present) with `ErrorResponse` payloads.
- **`.prefer(code, example)` (16 requests):** Because Prism is stateless, it does not evaluate token roles, disabled accounts, database existence/uniqueness, loyalty-discount arithmetic, or lifecycle/settlement guards. Scenarios select documented contract examples via `Prefer: code=..., example=...` to verify `403` (`disabledAccount`, `/staff`, `technicianDenied`, `technicianCancelDenied`), `404` (`NotFound`), `409` (`duplicateTicket`, `unapprovedRepairDenied`, `missingRepairCheckDenied`, `missingPickupDenied`, `shortPaymentDenied`, `reopenTerminalDenied`), and stage-specific `200` settlement examples (`readyOrderSilver`, `readyOrderHalfCentSilver`, `cancelledBeforeFindings`, `cancelledAfterFindings`, `cancelledInProgressGold`).

## Human decision

- [x] **GO** — Approve `spec/openapi.yaml` and `ADD-002`; unblock TDD backend implementation (`P1`–`P6`).
- [ ] **REVISE** — Record required contract adjustments below and re-run validation + Prism scenarios.
