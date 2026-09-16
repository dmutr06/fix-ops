# GIT-GATE Decision Record: GATE-01

## Incident Summary
- **Target Branch**: `main`
- **Source Branch**: `agent/discount-calculation`
- **Conflicting Files**: `src/order.rs`, `tests/order_test.rs`
- **Affected Logic**: `RepairOrder::calculate_discount(&self) -> f64`

## Competing Proposals
1. **Branch `agent/discount-calculation`**:
   - Implements loyalty tier percentage discount (`Silver`: 10%, `Gold`: 20%).
   - Applies discount exclusively to `labor_cost`.
2. **Branch `main` (human commit)**:
   - Implements tiered volume discount based on total gross cost (`parts_cost + labor_cost`).
   - Flat discount thresholds ($15 for >= $150, $30 for >= $300).

## Benchmark Evaluation against `spec/concept.md`
- `spec/concept.md` (Pricing and Settlement Rules) mandates:
  1. *Discount is determined by client loyalty tier (Silver: 10%, Gold: 20%).*
  2. *Discount applies to labor charges only; parts costs remain undiscounted.*
- The human change directly violates rule #2 by discounting gross totals (effectively discounting parts).
- The agent change directly adheres to both rules.

## Resolution
- Selected the loyalty-based discount logic from `agent/discount-calculation`.
- Justification: strict compliance with the approved baseline specification. Change acceptance was determined by specification alignment, not commit author or timestamp.
- Verified compilation and test pass via `cargo test` (3/3 passing).

## Deferred Questions for Lab 2 (SRS Phase)
- Should enterprise/bulk orders qualify for additional discounts beyond loyalty tiers?
- Does total order spend accelerate loyalty tier progression?
