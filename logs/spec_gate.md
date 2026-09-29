# SPEC-GATE Decision Record: SPEC-GATE-01

- **Date:** 2026-09-30
- **Decision:** PASS; the human approved SRS baseline 1.0.0 after reviewing the updated SKED consensus, structure, and four Markdown artifacts.
- **Baseline:** The Git commit containing this record and `spec/srs.md`, `spec/glossary.md`, `spec/traceability.md`, and `logs/sked_dialogue.md`.
**Source baseline:** `fad43ab` (Lab 1 GIT-GATE).

## Independent specification review

- Confirmed that the SRS describes the target internal web system, while the current Rust code is identified as a partial implementation.
- Checked the release boundary and found explicit exclusions for the customer portal, notifications, inventory, payment integrations, analytics, bulk discounts, and automatic loyalty progression.
- Checked consistency of role permissions, order states, transition guards, diagnostic-fee boundaries, loyalty rates, labor-only discount, final-tier timing, cent precision, half-up rounding, and full-payment completion across the SRS, glossary, scenarios, and matrix.
- Verified 23 distinct requirement IDs, one acceptance criterion per requirement, 17 distinct BDD scenario IDs, and 23 matching traceability rows with no unresolved references.
- Checked the SKED plan against the agreed knowledge ledger: 41/41 entries covered, including the one explicitly deferred question.
- Checked for placeholder text, undefined domain terms in core rules, and untestable language; no baseline blocker remained.
- Ran `cargo test --offline`: 3 integration tests passed. These tests verify the existing prototype only; they do not prove that the target SRS is implemented.

## Known implementation gaps

The current Rust prototype does not implement the staff web interface, role controls, full client/device records, approval and payment evidence, guarded state transitions, audit logging, backup/recovery, or performance target. Its `Cancelled` calculation always returns $30, whereas the approved SRS distinguishes cancellation before saved findings, after saved findings, and after repair starts. These are later implementation work, not exceptions to this specification.

## Change control

The approved Markdown files are the versioned baseline. Any later requirement change must be recorded as an addendum that names affected requirement IDs, acceptance criteria, BDD scenarios, and traceability rows before related implementation changes are accepted.
