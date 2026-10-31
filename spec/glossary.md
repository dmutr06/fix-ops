# Domain glossary

| Term | Definition |
|---|---|
| Client | A customer whose device is received for service: identifier, name, phone number, optional email, and loyalty tier. |
| Device | One serviced item linked to a Client: type (`Desktop`, `Laptop`, or `Server`), serial number, and reported symptoms. |
| RepairOrder | A service transaction with a unique ticket number, linked to one Client, one Device, and an assigned Technician. It records status, estimate, incurred charges, approval, repair check, payment, and pickup. |
| Receptionist | Staff role: client and device records, intake, client decisions, payment, pickup, and cancellation. |
| Technician | Staff role: diagnosis, estimates, repair work, incurred charges, and repair checks. |
| Administrator | Staff role with every Receptionist and Technician permission, plus staff accounts, loyalty tiers, and correction of any RepairOrder field. Business rules and audit still apply. |
| Estimate | Itemized expected parts and labor charges prepared before the client decides. Not an incurred cost. |
| Client decision | Approval or decline of the estimate, entered by staff with the decision time. |
| Used parts cost | Cost of parts actually used; never discounted. |
| Completed labor cost | Cost of repair work actually done; the loyalty discount applies to it. |
| Loyalty tier | `Standard` (0%), `Silver` (10%), or `Gold` (20%), set manually by an Administrator. The tier at final settlement controls the discount. |
| Diagnostic findings | The Technician's saved diagnosis result. After findings are saved, cancellation before repair costs the $30 diagnostic fee. |
| Diagnostic fee | Fixed $30 charge for cancelling after findings are saved but before repair starts. Never added once repair has started. |
| Settlement amount | Amount owed for a completed or cancelled order, per the pricing rules in [`srs.md`](srs.md). |
| Payment record | Payment amount and time entered by a Receptionist. Must equal the full settlement amount before completion. Payment is recorded, not processed. |
| Repair check | Technician's recorded confirmation that the repair is finished and checked. Required before `Ready`. |
| Pickup record | Receptionist's confirmation that the client collected the device. |
| Audit log | Record of each change: actor, time, record, field, previous value, and new value. |

**Cost** has two meanings: *estimated cost* is a proposal; *incurred cost* is used parts and completed labor. Only incurred costs enter settlement.

Order states (`Received`, `Diagnosing`, `InProgress`, `Ready`, `Completed`, `Cancelled`) and their transitions are defined in [SRS §3.1](srs.md#31-repair-order-lifecycle).
