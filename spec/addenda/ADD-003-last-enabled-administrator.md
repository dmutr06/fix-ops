# ADD-003: Preserve an enabled Administrator

**Changes:** SRS 1.0.2 → 1.0.3 · **Affects:** REQ-F-019, REQ-F-020

## Policy

1. Updating a staff account must not demote or disable the last currently enabled Administrator. The update returns `409 CONFLICT`; the staff account, its sessions, and its audit history remain unchanged.
2. Disabled Administrator accounts do not count toward the invariant.
3. Promoting an account to Administrator remains allowed when no enabled Administrator currently exists, so access can be restored.
4. Concurrent updates that demote or disable different Administrators are serialized; at most one may succeed if that would leave no enabled Administrator.

## Test impact

| Requirements | What to test | Scenarios |
|---|---|---|
| REQ-F-019, REQ-F-020 | Last enabled Administrator cannot be demoted or disabled; account, sessions, and audit stay unchanged. | BDD-021 |
| REQ-F-020 | Disabled accounts do not count; a remaining enabled Administrator permits demotion; promotion from zero enabled Administrators is allowed. | BDD-021 |
| REQ-F-020 | Concurrent demotions cannot both succeed and leave the service without an enabled Administrator. | Concurrent staff update test |
