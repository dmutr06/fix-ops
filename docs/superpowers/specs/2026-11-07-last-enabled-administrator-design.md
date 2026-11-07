# Keep one enabled Administrator

## Intent

Prevent staff-account changes from locking the workshop out of Administrator-only management. An enabled Administrator can disable or demote themselves or another Administrator today, leaving no account able to manage staff or correct orders.

## Scope and requirements

- Add SRS addendum ADD-003 and trace the rule to `PATCH /api/v1/staff/{staffId}`.
- Reject a change that would disable or demote the last enabled Administrator. The response is `409 CONFLICT`; the account, its sessions, and its audit history remain unchanged.
- Disabled Administrators do not count. An Administrator can still disable or demote an account when another enabled Administrator remains. Promoting an account to Administrator remains allowed, including recovery when a database has no enabled Administrator.
- Serialize concurrent demotions so two Administrators cannot both observe the other as available and remove the final account.

## Approaches

1. **Database trigger.** Central enforcement covers every SQL writer, but hides a role policy in trigger code and complicates the existing store transaction and test story.
2. **Serialized store transaction (selected).** Lock the target account, then take a transaction-scoped PostgreSQL advisory lock for the last-Administrator check. Count other enabled Administrators and perform the role update, session deletion, and audit insert in the same transaction. This fits the existing `StaffStore` boundary and closes the two-account race without another table or dependency.
3. **No invariant.** Keeps the current API unchanged but permits administrative lockout, so it does not meet the security goal.

## Constraints and non-goals

- Keep HTTP DTOs in the root crate, SQL in `fixops-infra`, and policy in the SRS.
- Reuse the existing `409 CONFLICT` error model and transaction/audit path.
- Do not add account recovery, a new role, a bootstrap bypass, or a dependency.
- Existing databases with no enabled Administrator can recover by promoting an account; the rule blocks only demoting/disabling an enabled Administrator when no other enabled Administrator remains.

## Acceptance checks

- Disabling or demoting the only enabled Administrator returns `409`; account, sessions and audit rows do not change.
- The same update succeeds when another enabled Administrator exists.
- Disabled Administrators do not satisfy the invariant.
- A role promotion to Administrator succeeds.
- Two concurrent attempts to demote the last two enabled Administrators cannot both succeed.
- Successful updates keep the existing session-revocation and audit behavior.

## Decision

The delegated gate selects approach 2. The implementation will serialize only the destructive transition from an enabled Administrator and will keep the check and write in one transaction.
