# Concept: Computer Service Management System

## Purpose
A lightweight management system for PC and electronics repair workshops. Tracks repair intake, diagnosis, labor, parts, and order settlement.

## Core Entities
- **Client**: Contact details, client ID, loyalty tier (`Standard`, `Silver`, `Gold`).
- **Device**: Serial number, device type (Desktop, Laptop, Server), reported symptoms.
- **RepairOrder**: Unique ticket ID, assigned technician, status, itemized charges.

## Lifecycle States
1. `Received`: Initial intake and physical check.
2. `Diagnosing`: Technician inspects hardware and estimates repair cost.
3. `InProgress`: Client approved estimate; repair is actively underway.
4. `Ready`: Repair finished and bench-tested; ready for pickup.
5. `Completed`: Client paid and collected device.
6. `Cancelled`: Client declined estimate or device deemed unrepairable.

## Pricing and Settlement Rules
- **Diagnostic Fee**: Fixed base fee of $30. Waived/credited if repair is approved and completed; charged if client cancels after diagnosis.
- **Total Calculation**: `Total = Parts Cost + Labor Cost - Discount`.
- **Discount Rules**:
  - Discount is determined by client loyalty tier (`Silver`: 10%, `Gold`: 20%).
  - Discount applies to labor charges only; parts costs remain undiscounted.
