# SKED dialogue summary for SRS 1.0.0

- **Source baseline:** `spec/concept.md` and `logs/git_gate.md` at `fad43ab`
- **Session:** `fix-ops-lab2-srs`; 9 initial dialogue rounds plus focused post-synthesis clarification
**Consensus:** Human-ended initial dialogue; 40 agreed knowledge entries, one deferred question, no contested entries. The initial K* and five-section structure were explicitly confirmed by the human; the focused clarifications were also answered by the human before baseline review.

## Initial frame and uncertainties

The model began with the workshop concept, its three domain entities, six order states, $30 diagnostic fee, labor-only loyalty discount, and two questions carried forward from GIT-GATE. It asked about release boundaries, role permissions, evidence of client approval, cancellation charges, extra discounts, loyalty progression, interface, data fields, lifecycle guards, and measurable quality targets.

## Human decisions

1. The first release covers client/device records and the full order cycle from intake through settlement. It excludes a customer portal, automated notifications, inventory management, payment-provider integrations, and analytics.
2. The roles are Receptionist, Technician, and Administrator, with the responsibilities listed in `spec/srs.md`. Staff record a client's estimate decision and its time; the customer does not act through a portal.
3. Cancellation costs $0 before diagnostic findings are saved and $30 after findings are saved but before repair starts. Once repair starts, the charge is used parts plus completed labor less the tier discount on labor, with no extra diagnostic fee.
4. There are no bulk or enterprise discounts in the first release. Automatic loyalty-tier progression based on spend is deferred, including the rule that could define it.
5. The staff interface is internal and web-based, with no external integrations. Normal order transitions, cancellation source states, approval, repair-check, payment, and pickup guards were confirmed.
6. The Receptionist records payment amount and time. The minimum fields for Client, Device, and RepairOrder were confirmed.
7. The accepted quality targets are 10 concurrent staff users, 10,000 orders, 95% of routine actions within 2 seconds, individual sign-ins and role controls, an audit log, daily backup, and restoration within 24 hours.
8. The human clarified that `Completed` requires payment of the full settlement amount, monetary values use whole cents, the diagnostic fee begins only after findings are saved, and fractional-cent discounts round to the nearest cent with exact halves upward.
9. A loyalty-tier change during an open order affects the final settlement: the Client's tier at calculation time is used.

## Model assumptions tested or corrected

- The existing Rust prototype charges $30 for every `Cancelled` state. The human specified three cancellation stages, so that prototype behavior is not the target rule.
- The model proposed a customer portal, notifications, inventory, payment integration, and analytics as possible features. The human excluded them from the first release.
- The GIT-GATE left additional bulk discounts and spend-driven loyalty progression open. The human rejected additional discounts for this release and explicitly deferred automatic progression.
- The model proposed a role/action split, a manual client-decision record, a state transition table, and minimum data fields. The human confirmed or refined each proposal before it entered the baseline.
- A draft review exposed an ambiguity between entering `Diagnosing` and completing diagnosis. The human placed the $30 fee boundary at saved findings, so cancellation during unfinished diagnosis remains $0.

## Convergence and approval

The human reviewed a compact synthesis, reported no missing or incorrect assumption, and ended the initial dialogue. The SKED ledger's stability metric alone was not used as proof of correctness. The initial K* and five-section SRS structure were confirmed. Post-synthesis clarifications were entered into the same ledger and structure; final entry coverage is 41/41 including the deferred question. The local SKED session preserves the detailed append-only ledger and dialogue. This repository summary is the durable audit artifact for the baseline.
