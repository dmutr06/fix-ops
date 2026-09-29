# Domain glossary

**Baseline:** Computer Service Management System SRS 1.0.0 candidate. These names are used consistently in requirements, scenarios, tickets, tests, and later documentation.

| Term | Definition |
|---|---|
| Client | A customer whose device is received for service. The record has an identifier, name, phone number, optional email, and loyalty tier. |
| Device | One serviced item linked to a Client. The record contains type (`Desktop`, `Laptop`, or `Server`), serial number, and client-reported symptoms. |
| RepairOrder | A service transaction identified by a unique ticket number and linked to one Client, one Device, and an assigned Technician. It records status, estimate, incurred charges, approval, repair check, payment, and pickup. |
| Receptionist | Staff role responsible for client and device records, intake, recording client decisions and payment, and pickup. |
| Technician | Staff role responsible for diagnosis, estimates, repair work, incurred charges, and repair checks. |
| Administrator | Staff role responsible for staff accounts, loyalty tiers, and repair-order corrections. |
| Estimate | Itemized expected parts and labor charges prepared before the client decides whether repair may start. An estimate is not an incurred cost. |
| Client decision | Approval or decline of the estimate, entered by staff together with the decision time. |
| Used parts cost | Cost of parts actually used on a repair order; unlike an estimated cost, it contributes to settlement. It is never discounted. |
| Completed labor cost | Cost of repair work actually completed; the loyalty discount applies to this amount. |
| Loyalty tier | Client category: `Standard` (0%), `Silver` (10%), or `Gold` (20%). In the first release an Administrator sets it manually. The tier at final settlement calculation controls the order's discount. |
| Diagnostic findings | Technician's saved result of diagnosing a device. Saving findings is the boundary after which cancellation before repair incurs the $30 diagnostic fee. |
| Diagnostic fee | Fixed $30 charge for cancellation after findings are saved but before repair starts. It is not an additional charge once repair has started or a repair is completed. |
| Settlement amount | Amount owed for a completed or cancelled order under the pricing rules in `srs.md`. |
| Payment record | Receptionist-entered payment amount and time. The amount must cover the full settlement amount before completion. The system records payment but does not process it through a provider. |
| Repair check | Recorded confirmation by a Technician that repair work is finished and checked before `Ready`. |
| Pickup record | Receptionist-entered confirmation that the client collected the device. |
| Audit log | Record of changes with actor, time, affected record and field, previous value, and new value. |
| Received | Order created at intake; diagnosis has not started. Cancellation charge is $0. |
| Diagnosing | Technician is diagnosing the device and preparing an estimate. Cancellation charge is $0 before findings are saved and $30 afterward. |
| InProgress | Client approved the estimate and repair has started. Cancellation charge is based on incurred parts and labor less the labor-only loyalty discount. |
| Ready | Repair check is complete and the device awaits payment and pickup. Cancellation is no longer available. |
| Completed | Payment and pickup are recorded; the order is closed. |
| Cancelled | Terminal state reached from `Received`, `Diagnosing`, or `InProgress`. |

The same word **cost** has two uses: *estimated cost* is a proposed charge, while *incurred cost* refers to used parts and completed labor. Only incurred costs enter settlement after repair starts.

All monetary values use whole USD cents and display with two decimal places. A fractional-cent loyalty discount rounds to the nearest cent, with an exact half cent rounded upward, before it is subtracted from settlement.
