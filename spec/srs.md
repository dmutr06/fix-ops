# Software Requirements Specification: Computer Service Management System

- **Version:** 1.0.0
- **Status:** Approved baseline (SPEC-GATE-01)
**Source:** `spec/concept.md`, `logs/git_gate.md`, and the confirmed Lab 2 SKED consensus.

## 1. Purpose and scope

This specification defines the first target release of an internal web system for a PC and electronics repair workshop. Its readers are the course reviewer and future implementers. The existing Rust program is a partial prototype; these requirements describe the target system and do not claim that the current code implements it.

The release covers client and device records, repair-order intake, diagnosis, estimates, recording client approval, repair work, pickup, and settlement. It does not include a customer portal, automated notifications, inventory management, payment-provider integrations, analytics, bulk or enterprise discounts, or automatic loyalty-tier progression based on spend. The last topic remains a deferred future decision.

## 2. Actors, records, and interface

Only staff use the internal web interface. Each staff member has an individual account and one of three roles:

| Role | Authorized responsibilities |
|---|---|
| Receptionist | Maintain clients and devices; receive orders; record client decisions, payments, and pickup. |
| Technician | Diagnose devices; prepare estimates; record parts, labor, and repair checks. |
| Administrator | Manage staff accounts and loyalty tiers; correct repair orders. |

The interface shall provide the screens and actions needed for these responsibilities. It has no external integration in this release. The required record fields and domain terms are defined in [`glossary.md`](glossary.md).

## 3. Domain rules

### 3.1 Repair-order lifecycle

The normal path is `Received → Diagnosing → InProgress → Ready → Completed`. `Cancelled` is allowed from `Received`, `Diagnosing`, or `InProgress` only. `Completed` and `Cancelled` are terminal; `Ready` can only move to `Completed`. Starting `InProgress` requires a recorded positive client decision on the estimate. Entering `Ready` requires a recorded repair check. Entering `Completed` requires a recorded full payment and pickup.

| From | Allowed next state | Guard |
|---|---|---|
| Received | Diagnosing | Technician starts diagnosis. |
| Received | Cancelled | No diagnosis has started. |
| Diagnosing | InProgress | Client approved the estimate; decision and time are recorded. |
| Diagnosing | Cancelled | Client declined or repair is not feasible; charge depends on whether findings were saved. |
| InProgress | Ready | Repair check recorded. |
| InProgress | Cancelled | Work stops; incurred costs remain recorded. |
| Ready | Completed | Receptionist recorded payment equal to the full settlement amount, payment time, and pickup. |

All other state transitions shall be rejected. The table defines business state behavior, not the present Rust prototype's permissive field assignment.

### 3.2 Pricing

Amounts are in USD, represented in whole cents and displayed with two decimal places. The loyalty tier is `Standard` (0%), `Silver` (10%), or `Gold` (20%). Use the Client's tier when the final settlement amount is calculated, including if an Administrator changed it while the order was open. The tier discount applies only to labor; parts are never discounted. Calculate the discount from completed labor, round a fractional cent to the nearest cent with an exact half cent rounded upward, then subtract it. For a completed repair, `total cents = used parts cents + completed labor cents − rounded labor discount cents`. The $30 diagnostic fee is credited or waived on a completed repair and is not added to this total.

| Cancellation point | Charge |
|---|---|
| Before diagnostic findings are saved (`Received` or `Diagnosing`) | $0 |
| After diagnostic findings are saved, before repair starts (`Diagnosing`) | $30 diagnostic fee |
| After repair starts (`InProgress`) | Used parts cost + completed labor cost − labor-only loyalty discount; no separate diagnostic fee |

The system records settlement amounts but does not process a payment through a provider. No other discount is available in this release.

## 4. Functional requirements and acceptance criteria

Each criterion is an observable condition for its requirement. Examples and edge cases are in §6. A denied action must leave the affected record unchanged.

| ID | Requirement | Acceptance criterion |
|---|---|---|
| REQ-F-001 | The system shall require an individual staff sign-in before exposing staff records or actions. | AC-F-001: An unauthenticated request cannot read or change a client, device, or order; a signed-in user can access permitted actions. |
| REQ-F-002 | The system shall restrict staff actions according to the role table in §2. | AC-F-002: A user can perform each listed action for their role; a user without that responsibility is denied and no record changes. |
| REQ-F-003 | A Receptionist shall create and update a Client with a name and phone number; email is optional. | AC-F-003: A record with name and phone is saved and can be reopened; missing either required value prevents saving. |
| REQ-F-004 | A Receptionist shall create and update a Device with type, serial number, and reported symptoms. | AC-F-004: All three values persist and can be retrieved; missing any prevents saving. |
| REQ-F-005 | A Receptionist shall create a RepairOrder linked to one Client and one Device, with a unique ticket number, assigned Technician, `Received` state, and itemized charge fields. | AC-F-005: Creation saves all required links and fields in `Received`; a duplicate ticket number or missing required link is rejected. |
| REQ-F-006 | A Technician shall move an order from `Received` to `Diagnosing` and record diagnostic findings. | AC-F-006: The transition and findings persist; the $30 diagnostic charge does not arise until findings have been saved. |
| REQ-F-007 | A Technician shall prepare an estimate with itemized expected parts and labor charges after recording diagnostic findings. | AC-F-007: The estimate can be viewed with separate parts and labor amounts after findings are saved and before a client decision is recorded. |
| REQ-F-008 | A Receptionist shall record the client's estimate decision and its time. | AC-F-008: The order shows approval or decline and the recorded time; a decision without a time is rejected. |
| REQ-F-009 | The system shall permit `Diagnosing → InProgress` only when the estimate has recorded client approval. | AC-F-009: Approved orders can start repair; declined or undecided orders cannot enter `InProgress`. |
| REQ-F-010 | A Technician shall record used parts and completed labor as separate itemized charges during repair. | AC-F-010: The order exposes separate used-parts and completed-labor totals; estimate-only amounts do not count as incurred costs. |
| REQ-F-011 | A Technician shall record a repair check before moving `InProgress → Ready`. | AC-F-011: A recorded check permits `Ready`; without one, the transition is rejected. |
| REQ-F-012 | A Receptionist shall record the payment amount and time for a `Ready` order. | AC-F-012: Both values persist; a payment record missing either value is rejected. |
| REQ-F-013 | A Receptionist shall record pickup and complete an order only after full payment and pickup have been recorded. | AC-F-013: A `Ready` order becomes `Completed` only when the recorded payment equals the full settlement amount and pickup is recorded; a short payment or missing pickup prevents completion. |
| REQ-F-014 | The system shall enforce the transition table in §3.1 and allow cancellation only from its stated source states. | AC-F-014: Each listed transition succeeds when its guard holds; every other transition, including cancellation from `Ready` or `Completed`, is rejected. |
| REQ-F-015 | The system shall calculate cancellation charges by the cancellation point in §3.2. | AC-F-015: The charge is $0 before saved findings, $30 after saved findings but before repair, or incurred costs after repair begins; the repair-stage charge has no extra diagnostic fee. |
| REQ-F-016 | The system shall calculate the completed repair total with the Client's tier at final settlement, a labor-only discount, and the rounding rule in §3.2. | AC-F-016: At parts=$100 and labor=$100, totals are $200 Standard, $190 Silver, and $180 Gold; a Silver discount on $0.05 labor rounds from half a cent to $0.01; a tier change during an open order affects its later final settlement. |
| REQ-F-017 | An Administrator shall set a Client's loyalty tier manually to Standard, Silver, or Gold. | AC-F-017: An authorized change persists; order spend alone does not change the tier. |
| REQ-F-018 | An Administrator shall correct a RepairOrder while preserving an audit record of the change. | AC-F-018: A correction persists and its actor, time, affected field, previous value, and new value are retrievable in the audit log. |
| REQ-F-019 | An Administrator shall create, update, and disable individual staff accounts with an assigned role. | AC-F-019: The assigned role controls available actions; a disabled account cannot sign in. |

## 5. Nonfunctional requirements and acceptance criteria

| ID | Requirement | Acceptance criterion |
|---|---|---|
| REQ-NF-001 | With 10 concurrent staff users and 10,000 repair orders, at least 95% of routine web actions shall complete within 2 seconds. | AC-NF-001: In a test with that data and concurrency, at least 95% of client/device search, order open/save, and allowed status-transition actions return a result within 2 seconds, measured from request to visible result. |
| REQ-NF-002 | Staff access shall use individual accounts and role restrictions. | AC-NF-002: Each recorded change identifies one staff account; tests of every role's allowed and denied actions match §2. |
| REQ-NF-003 | The system shall retain an audit log of changes to clients, devices, orders, loyalty tiers, and staff accounts. | AC-NF-003: For each tested change, the log identifies actor, time, record, field, previous value, and new value; the entry remains available after the record is reopened. |
| REQ-NF-004 | The system shall create a backup at least daily and support restoration within 24 hours. | AC-NF-004: A backup exists for each elapsed 24-hour period; in a restoration exercise, the backed-up records can be queried within 24 hours of starting recovery. |

## 6. BDD scenarios

Scenario IDs are stable references for [`traceability.md`](traceability.md). `Given` describes the starting state, `When` the staff action, and `Then` the observable result.

**BDD-001 — Intake with required records** (`REQ-F-003`–`005`)

```gherkin
Given a signed-in Receptionist and valid client and device details
When the Receptionist creates a repair order with a new ticket number and assigned Technician
Then the client and device are linked to an order in Received state
```

**BDD-002 — Missing intake data** (`REQ-F-003`–`005`)

```gherkin
Given a signed-in Receptionist
When the Receptionist saves a device without a serial number or an order with an existing ticket number
Then the save is rejected and no incomplete or duplicate record is created
```

**BDD-003 — Approval gates repair** (`REQ-F-007`–`009`)

```gherkin
Given an order in Diagnosing with saved diagnostic findings and an itemized estimate
When a Receptionist records client approval and its time and a Technician starts repair
Then the order enters InProgress and the approval remains visible
```

**BDD-004 — No approval, no repair** (`REQ-F-008`, `REQ-F-009`, `REQ-F-014`)

```gherkin
Given an order in Diagnosing without recorded client approval
When a Technician requests InProgress
Then the transition is rejected and the order remains Diagnosing
```

**BDD-005 — Cancellation before diagnosis** (`REQ-F-014`, `REQ-F-015`)

```gherkin
Given an order in Received with no diagnosis started
When the order is cancelled
Then the order becomes Cancelled and its charge is $0
```

**BDD-006 — Declined estimate after diagnosis** (`REQ-F-008`, `REQ-F-014`, `REQ-F-015`)

```gherkin
Given an order in Diagnosing with saved diagnostic findings and an estimate
When a Receptionist records that the client declined and cancels the order
Then the order becomes Cancelled and its charge is $30
```

**BDD-007 — Cancellation after work starts** (`REQ-F-010`, `REQ-F-015`, `REQ-F-016`)

```gherkin
Given a Gold client's order in InProgress with $100 used parts and $50 completed labor
When the order is cancelled
Then its charge is $140 and no separate diagnostic fee is added
```

**BDD-008 — Parts are never discounted** (`REQ-F-016`)

```gherkin
Given a Silver client's order with $100 used parts and $100 completed labor
When the completed repair total is calculated
Then the labor discount is $10 and the total is $190
```

**BDD-009 — Repair check gates readiness** (`REQ-F-011`, `REQ-F-014`)

```gherkin
Given an order in InProgress without a recorded repair check
When a Technician requests Ready
Then the transition is rejected until the check is recorded
```

**BDD-010 — Full payment and pickup gate completion** (`REQ-F-012`–`014`)

```gherkin
Given an order in Ready with a recorded full payment amount and time but no pickup record
When a Receptionist requests Completed
Then the transition is rejected until pickup is recorded
```

**BDD-011 — Role restriction** (`REQ-F-002`, `REQ-F-017`, `REQ-NF-002`)

```gherkin
Given a signed-in Technician and an existing order
When the Technician tries to change a client's loyalty tier
Then the change is denied and the tier remains unchanged
```

**BDD-012 — Short payment does not complete an order** (`REQ-F-012`, `REQ-F-013`)

```gherkin
Given an order in Ready with a $190 settlement amount and recorded pickup
When a Receptionist records a $100 payment and requests Completed
Then the transition is rejected and the order remains Ready
```

**BDD-013 — Cancellation during unfinished diagnosis** (`REQ-F-006`, `REQ-F-015`)

```gherkin
Given an order in Diagnosing without saved diagnostic findings
When the order is cancelled
Then the order becomes Cancelled and its charge is $0
```

**BDD-014 — Half-cent discount rounds upward** (`REQ-F-016`)

```gherkin
Given a Silver client's completed repair with $0 used parts and $0.05 completed labor
When the completed repair total is calculated
Then the $0.005 discount rounds to $0.01 and the total is $0.04
```

**BDD-015 — Tier at final settlement** (`REQ-F-016`, `REQ-F-017`)

```gherkin
Given an open order with $100 used parts and $100 completed labor while its Client is Silver
When an Administrator changes the Client to Gold before final settlement is calculated
Then the completed repair total is $180 using the Gold labor discount
```

**BDD-016 — Administrator correction is audited** (`REQ-F-018`, `REQ-NF-003`)

```gherkin
Given an existing order with a recorded labor charge and a signed-in Administrator
When the Administrator corrects that charge
Then the new charge is saved and an audit entry shows the actor, time, order, field, old value, and new value
```

**BDD-017 — Staff sign-in is required** (`REQ-F-001`)

```gherkin
Given a visitor without a staff sign-in
When the visitor requests a client record or a repair order
Then access is denied and no record contents are shown
```

## 7. Constraints, open point, and baseline control

The interface is staff-only and browser-based. Payment is recorded, not processed. This SRS does not prescribe a storage engine, framework, or API. Automatic progression of loyalty tiers from accumulated spend is deferred; no rule for it is implied by this baseline. Any later change to an approved requirement shall be recorded as an addendum that identifies affected requirement IDs and traceability links.

The SPEC-GATE decision and baseline commit are recorded in `logs/spec_gate.md` after human review. This document is the target behavior; discrepancies with the current Rust prototype are implementation gaps, not exceptions to the requirements.
