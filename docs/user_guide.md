# User guide

For workshop staff. Terms follow the [glossary](../spec/glossary.md). Daily work happens in the [web app](#web-app); the sections after it describe the same steps through the API, whose requests and fields are in [`openapi.yaml`](../spec/openapi.yaml).

## Web app

Open `http://127.0.0.1:8080` on a computer, tablet or phone and sign in. **Sign out** ends the session at once.

| Page | Who | What |
|---|---|---|
| Queue | everyone | Your next work. Receptionist: orders *awaiting client decision* and *ready for payment and pickup*. Technician: *my open orders*. Administrator: all *open orders*. |
| Orders | everyone | Every order, by status or ticket number. |
| Clients | everyone | Find a client. Receptionists add a client (**New client**), then on the client's page **Add device** and **New order** for that device with a ticket number and a Technician. |
| Order | everyone | Details, the **Actions** your role may take at this step, and the **History**. Amounts are entered in dollars, such as `12.50`. |
| Staff | Administrator | **New staff account**, change a role, **Disable** or **Enable**. |

On an order, the Technician uses **Start diagnosis**, **Save findings**, **Save estimate**, **Start repair** and **Mark ready**; the Receptionist uses **Approve** or **Decline**, **Record payment**, **Record pickup**, **Complete** and **Cancel order**. An Administrator sees all of them, plus **Correct** on the order and the loyalty tier on the client's page. A blocked step shows the reason, for example a missing repair check, and changes nothing.

## Sign in (API)

```bash
API=http://127.0.0.1:8080/api/v1
TOKEN=$(curl -s "$API/sessions" -H 'Content-Type: application/json' \
  -d '{"username":"anna","password":"her-password"}' | jq -r .token)
curl -s "$API/orders?status=Ready" -H "Authorization: Bearer $TOKEN"
```

A session lasts 12 hours. Send the token as `Authorization: Bearer` on every other request; `DELETE /sessions/current` signs out. Lists take `limit` (up to 100) and `offset` and return the total in `X-Total-Count`. Amounts are whole cents: `12550` is $125.50.

## Who does what

| Task | Request | Receptionist | Technician | Administrator |
|---|---|:-:|:-:|:-:|
| Find and read clients, devices, orders, audit log | `GET /clients?q=`, `/devices?client_id=&q=`, `/orders?status=&client_id=&ticket_number=`, `/audit-logs?record_type=&record_id=` | ✓ | ✓ | ✓ |
| Add or edit a client or device | `POST /clients`, `PATCH /clients/{id}`, `POST /devices`, `PATCH /devices/{id}` | ✓ | | ✓ |
| Open a repair order | `POST /orders` | ✓ | | ✓ |
| Diagnose and estimate | `PUT /orders/{id}/diagnosis`, `PUT /orders/{id}/estimate` | | ✓ | ✓ |
| Record the client's decision | `PUT /orders/{id}/decision` | ✓ | | ✓ |
| Repair: charges, repair check, `Ready` | `PUT /orders/{id}/repair` | | ✓ | ✓ |
| Payment, pickup, complete, cancel | `PUT /orders/{id}/payment`, `PUT /orders/{id}/pickup`, `POST /orders/{id}/complete`, `POST /orders/{id}/cancel` | ✓ | | ✓ |
| Loyalty tier | `PUT /clients/{id}/loyalty-tier` | | | ✓ |
| Staff list, for example Technicians to assign | `GET /staff?role=Technician&enabled=true` | ✓ | ✓ | ✓ |
| Staff accounts | `POST /staff`, `PATCH /staff/{id}` | | | ✓ |
| Correct any order field; lifecycle, payment and pricing rules still apply | `PATCH /orders/{id}` | | | ✓ |

## A repair, step by step

| # | Who | Request body | Order afterwards |
|---|---|---|---|
| 1 | Receptionist | `POST /orders` `{"ticket_number","client_id","device_id","assigned_technician_id"}` | `Received` |
| 2 | Technician | `PUT …/diagnosis` `{}` to start, then `{"diagnostic_findings":"…"}` | `Diagnosing`, findings saved |
| 3 | Technician | `PUT …/estimate` `{"parts_cents","labor_cents"}` | estimate saved |
| 4 | Receptionist | `PUT …/decision` `{"decision":"Approved","decided_at":"2026-11-21T10:00:00Z"}` | approval recorded |
| 5 | Technician | `PUT …/repair` `{"status":"InProgress","used_parts_cents","completed_labor_cents"}` | `InProgress` |
| 6 | Technician | `PUT …/repair` `{"status":"Ready","repair_checked":true}` | `Ready`; `settlement_cents` is the amount due |
| 7 | Receptionist | `PUT …/payment` `{"amount_cents","paid_at"}` and `PUT …/pickup` `{"picked_up_at"}` | payment and pickup recorded |
| 8 | Receptionist | `POST …/complete` | `Completed` |

The device must belong to the client, and the assignee must be a Technician. Findings and the estimate can be revised until the client decides. A declined estimate ends in cancellation.

## Money

- `settlement_cents` = used parts + completed labor − loyalty discount. The discount is on labor only: Silver 10%, Gold 20%, half a cent rounds up.
- The client's tier at completion or cancellation counts. An open order shows the amount at the current tier.
- Cancelling: $0 before findings are saved, $30 after findings and before repair, incurred charges after repair starts. `Ready` orders cannot be cancelled.
- Payment must equal `settlement_cents` exactly before completion. FixOps records payments; it does not take them.

## Errors

Every error has a `code` and a `message`; most also have `details` naming the field or rule.

| Status, `code` | Meaning | What to do |
|---|---|---|
| `400 VALIDATION_ERROR` | A field is missing, unknown or invalid; the message or `details.field` names it | Fix the field |
| `401 UNAUTHENTICATED` | No token, or it expired | Sign in again |
| `403 FORBIDDEN` | Your role may not do this, or the account is disabled | Ask a colleague with the role |
| `404 NOT_FOUND` | The record or a linked record does not exist | Check the id |
| `409 CONFLICT` | Ticket number or username taken, device of another client, assignee not a Technician, or the last enabled Administrator | Change the value |
| `409 GUARD_VIOLATION` | A lifecycle rule blocks the step (`details.guard`: `client_decision.Approved`, `repair_checked`, `payment`, `pickup`), or someone saved the record first | Record the missing step, or reload and retry |

A rejected request changes nothing.

## Audit log

Every change is logged per field with who, when, and the previous and new value: `GET /audit-logs?record_type=RepairOrder&record_id=42`. Passwords are never shown. Entries cannot be edited or deleted.

## Staff accounts (Administrator)

`POST /staff` `{"username","password","role"}` creates an account; passwords need at least 8 characters. `PATCH /staff/{id}` `{"role"}` or `{"enabled":false}` changes it; disabling signs the person out at once. The last enabled Administrator cannot be disabled or demoted.
