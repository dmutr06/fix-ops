# Staff API validation walkthrough

This is a human checklist for the FixOps **API**. The frontend is not implemented. No human validation has been recorded: leave results **pending** until an operator runs the checks. Use a disposable local database and unique ticket numbers; the bootstrap Administrator is created only when no staff accounts exist.

## Start the service and sign in

From the repository root, start the Compose project as `fix-ops`, then run the API with the bootstrap credentials set:

```sh
docker compose -p fix-ops up -d --wait
export FIXOPS_BOOTSTRAP_ADMIN_USERNAME=admin
export FIXOPS_BOOTSTRAP_ADMIN_PASSWORD='change-this-local-password'
cargo run
```

Migrations run at startup. In another terminal, set the API root, get the bootstrap Administrator token, and have that Administrator create and sign in as the other two roles. Replace sample passwords before use; staff passwords must be at least 8 characters.

```sh
API=http://127.0.0.1:8080/api/v1
ADMIN_TOKEN=$(curl -fsS "$API/sessions" -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"change-this-local-password"}' | jq -r .token)
curl -fsS "$API/staff" -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"username":"frontdesk","password":"frontdesk-local-123","role":"Receptionist"}'
curl -fsS "$API/staff" -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"username":"tech","password":"tech-local-123","role":"Technician"}'
RECEPTIONIST_TOKEN=$(curl -fsS "$API/sessions" -H 'Content-Type: application/json' \
  -d '{"username":"frontdesk","password":"frontdesk-local-123"}' | jq -r .token)
TECHNICIAN_TOKEN=$(curl -fsS "$API/sessions" -H 'Content-Type: application/json' \
  -d '{"username":"tech","password":"tech-local-123"}' | jq -r .token)
```

Keep the three tokens private. Send them as `Authorization: Bearer $TOKEN`; do not paste them into logs or validation notes. URLs below are relative to `$API`.

## One complete happy path

Run with the three variables above in scope. Each response is printed so the operator can note the generated `id` values. The example uses Standard tier, $100.00 in parts and $100.00 labor: expect `used_parts_cents: 10000`, `completed_labor_cents: 10000`, `labor_discount_cents: 0`, and `settlement_cents: 20000`.

```sh
curl -fsS "$API/clients" -H "Authorization: Bearer $RECEPTIONIST_TOKEN" -H 'Content-Type: application/json' \
  -d '{"name":"Walkthrough Client","phone":"+1-555-0198"}'
# Set CLIENT_ID to the returned id.
curl -fsS "$API/devices" -H "Authorization: Bearer $RECEPTIONIST_TOKEN" -H 'Content-Type: application/json' \
  -d "{\"client_id\":$CLIENT_ID,\"device_type\":\"Laptop\",\"serial_number\":\"WALK-001\",\"reported_symptoms\":\"No power\"}"
# Set DEVICE_ID to the returned id and choose an unused ticket number.
curl -fsS "$API/orders" -H "Authorization: Bearer $RECEPTIONIST_TOKEN" -H 'Content-Type: application/json' \
  -d "{\"ticket_number\":\"WALK-2026-001\",\"client_id\":$CLIENT_ID,\"device_id\":$DEVICE_ID,\"assigned_technician_id\":$TECHNICIAN_ID}"
# Set ORDER_ID to the returned id.
curl -fsS -X PUT "$API/orders/$ORDER_ID/diagnosis" -H "Authorization: Bearer $TECHNICIAN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"diagnostic_findings":"Loose power connector"}'
curl -fsS -X PUT "$API/orders/$ORDER_ID/estimate" -H "Authorization: Bearer $TECHNICIAN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"parts_cents":10000,"labor_cents":10000}'
curl -fsS -X PUT "$API/orders/$ORDER_ID/decision" -H "Authorization: Bearer $RECEPTIONIST_TOKEN" -H 'Content-Type: application/json' \
  -d '{"decision":"Approved","decided_at":"2026-11-07T10:00:00Z"}'
curl -fsS -X PUT "$API/orders/$ORDER_ID/repair" -H "Authorization: Bearer $TECHNICIAN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"status":"InProgress","used_parts_cents":10000,"completed_labor_cents":10000}'
curl -fsS -X PUT "$API/orders/$ORDER_ID/repair" -H "Authorization: Bearer $TECHNICIAN_TOKEN" -H 'Content-Type: application/json' \
  -d '{"status":"Ready","repair_checked":true}'
curl -fsS -X PUT "$API/orders/$ORDER_ID/payment" -H "Authorization: Bearer $RECEPTIONIST_TOKEN" -H 'Content-Type: application/json' \
  -d '{"amount_cents":20000,"paid_at":"2026-11-07T11:00:00Z"}'
curl -fsS -X PUT "$API/orders/$ORDER_ID/pickup" -H "Authorization: Bearer $RECEPTIONIST_TOKEN" -H 'Content-Type: application/json' \
  -d '{"picked_up_at":"2026-11-07T11:05:00Z"}'
curl -fsS -X POST "$API/orders/$ORDER_ID/complete" -H "Authorization: Bearer $RECEPTIONIST_TOKEN"
curl -fsS "$API/orders/$ORDER_ID" -H "Authorization: Bearer $RECEPTIONIST_TOKEN"
```

The create responses include IDs in JSON; assign those to `CLIENT_ID`, `DEVICE_ID`, `TECHNICIAN_ID` (from the staff create response), and `ORDER_ID` before continuing. A completed order should retain its payment and pickup records and show `status: "Completed"`.

## BDD scenario checklist

For each row, prepare a separate suitable order/client unless the scenario explicitly changes an existing one. Record the actual HTTP result and observations in the blank column; do not mark a scenario passed from this guide alone.

| Scenario | Concrete API action | Observable expected outcome | Human result |
|---|---|---|---|
| BDD-001 | As Receptionist, create a client, its device, then an order assigned to a Technician. | Order is `Received`; `client_id`, `device_id`, and `assigned_technician_id` link to the created records. | Pending |
| BDD-002 | Omit `serial_number` when creating a device; separately submit an already-used `ticket_number` to `POST /orders`. | Missing field returns `400 VALIDATION_ERROR`; duplicate ticket returns `409 CONFLICT`; neither creates a record. | Pending |
| BDD-003 | Technician saves diagnosis and itemized estimate; Receptionist records `Approved` with `decided_at`; Technician requests `InProgress`. | `200`; status is `InProgress`, and approval and timestamp remain readable. | Pending |
| BDD-004 | On a Diagnosing order with no approval, Technician requests `InProgress`. | `409 GUARD_VIOLATION`; order remains `Diagnosing`, unchanged. | Pending |
| BDD-005 | Receptionist cancels a `Received` order with no diagnosis. | `200`; `Cancelled`, `settlement_cents: 0` ($0.00). | Pending |
| BDD-006 | Save findings and estimate, record client `Declined`, then Receptionist cancels in `Diagnosing`. | `200`; `Cancelled`, `settlement_cents: 3000` ($30.00 diagnostic fee). | Pending |
| BDD-007 | Set client to Gold; in `InProgress`, record $100.00 parts (`10000`) and $50.00 labor (`5000`); cancel. | `200`; `Cancelled`, `labor_discount_cents: 1000` ($10.00), `settlement_cents: 14000` ($140.00); no extra diagnostic fee. | Pending |
| BDD-008 | Set client to Silver; record $100.00 parts and $100.00 labor, repair check, and `Ready`. | `labor_discount_cents: 1000` ($10.00); `settlement_cents: 19000` ($190.00); parts are not discounted. | Pending |
| BDD-009 | From `InProgress` without `repair_checked`, Technician requests `Ready`; then set the check and retry. | First request returns `409 GUARD_VIOLATION`, state stays `InProgress`; checked request succeeds to `Ready`. | Pending |
| BDD-010 | On `Ready`, record full payment and request `Completed` before recording pickup; then record pickup and retry. | First completion returns `409 GUARD_VIOLATION`, still `Ready`; after pickup it completes. | Pending |
| BDD-011 | Technician calls `PUT /clients/{clientId}/loyalty-tier`. | `403 FORBIDDEN`; read the client and confirm the tier did not change. | Pending |
| BDD-012 | Use a Ready order with `settlement_cents: 19000`; record $100.00 (`amount_cents: 10000`) and pickup, then request completion. | `409 GUARD_VIOLATION`; remains `Ready`, with payment and pickup readable. | Pending |
| BDD-013 | Start diagnosis but save no findings; Receptionist cancels. | `200`; `Cancelled`, `settlement_cents: 0` ($0.00). | Pending |
| BDD-014 | Set client to Silver; record $0.00 parts and $0.05 labor (`completed_labor_cents: 5`) and finish repair. | Half-cent discount rounds up: `labor_discount_cents: 1` ($0.01), `settlement_cents: 4` ($0.04). | Pending |
| BDD-015 | With an open order at Silver and $100.00 parts plus $100.00 labor, Administrator changes tier to Gold before final settlement. | Read/calculated settlement uses Gold: $20.00 discount (`2000` cents), total $180.00 (`18000` cents). | Pending |
| BDD-016 | Administrator corrects `completed_labor_cents` with `PATCH /orders/{orderId}`; reopen the order with `GET`, then query `/audit-logs?record_type=RepairOrder&record_id={id}`. | Corrected value persists; audit entry shows Administrator actor, timestamp, field, previous and new values alongside earlier entries. | Pending |
| BDD-017 | Without a token, request `GET /clients/{clientId}` and `GET /orders/{orderId}`. | Both return `401 UNAUTHENTICATED`; no record contents are returned. | Pending |
| BDD-018 | Receptionist and Administrator each cancel separate eligible orders in `Received`, `Diagnosing`, and `InProgress`. | All six authorized requests succeed; status is `Cancelled`, with $0.00 before findings, $30.00 after findings before repair, or the incurred-cost formula after repair starts. | Pending |
| BDD-019 | Technician attempts `POST /orders/{orderId}/cancel` on an eligible order. | `403 FORBIDDEN`; GET confirms order and audit history are unchanged. | Pending |
| BDD-020 | Administrator performs a valid Receptionist action (create client) and Technician action (diagnosis or repair); also attempt guarded transitions without approval/check/payment/pickup and correct a terminal order. | Allowed actions succeed and audit actor is Administrator; failed guards remain `409` with no change. Valid terminal correction is audited; an attempt to reopen a terminal status is rejected and leaves it terminal. | Pending |

For every denial or failed guard, read the affected record afterward and confirm it is unchanged; query the audit log to confirm no change entry was added. For a successful update, verify the record again and inspect its audit entry.

## Logs and pending operational observations

The service writes structured logs to the console and daily JSON-lines files under `var/log` by default. In another terminal, inspect live console output from the `cargo run` terminal or follow today’s file:

```sh
tail -f "var/log/fixops.log.$(date +%F)"
```

Correlate requests using the response `x-request-id` and log fields `request_id`, `method`, `path`, `status`, and `latency_ms`. For expected authorization checks, observe `request rejected` (`UNAUTHENTICATED` / `FORBIDDEN`), `action denied`, and `request finished`; for successful changes, inspect `record changed`. Logs must not contain bearer tokens or changed values.

- API scenarios BDD-001–020: **pending human validation**.
- Browser workflow and timing against the 2-second / 95th-percentile requirement: **pending**; there is no implemented frontend to exercise.
- Daily backup existence and restore-within-24-hours exercise: **pending operational observation**; this walkthrough does not claim either has been performed.
