# Specification discovery summary (SRS 1.0.0)

Source: [`concept.md`](../spec/concept.md) and [GIT-GATE](git_gate.md). Nine dialogue rounds plus follow-up clarifications. 41 agreed points, one deferred, none disputed.

## Decisions

1. **Scope:** client/device records and the full order cycle through settlement. Excluded: customer portal, notifications, inventory, payment integration, analytics.
2. **Roles:** Receptionist, Technician, Administrator (see SRS §2). Staff enter the client's estimate decision and its time.
3. **Cancellation:** $0 before findings are saved; $30 after findings but before repair; after repair starts, used parts + completed labor − labor discount, with no extra fee.
4. **Discounts:** no bulk or enterprise discounts. Automatic tier progression is deferred.
5. **Interface:** internal web app, no external integrations. Transitions and guards confirmed.
6. **Records:** the Receptionist records payment amount and time. Minimum Client, Device, and RepairOrder fields confirmed.
7. **Quality:** 10 users, 10,000 orders, 95% of actions under 2 s, individual sign-in, roles, audit log, daily backup, restore within 24 h.
8. **Money:** `Completed` requires full payment; whole cents; half-cent discounts round up.
9. **Tier timing:** the client's tier at final settlement is used.

## Corrected assumptions

- The prototype's flat $30 cancellation fee was replaced by the three-stage rule.
- Proposed extras (portal, notifications, inventory, payments, analytics) were excluded.
- The open questions from GIT-GATE were resolved: no extra discounts, and tier progression is deferred.
- The $30 fee boundary was set at *saved findings*, so cancelling during unfinished diagnosis costs $0.
