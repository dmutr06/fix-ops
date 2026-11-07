# Backend SRS review fixes

Requirements: REQ-F-005, REQ-F-015, REQ-F-016, REQ-F-018, REQ-F-019, REQ-NF-003.
Tickets: FIX-55, FIX-56, FIX-57.

1. Reproduce stale staff writes, terminal metadata repricing, and intake assignee races with failing tests; commit tests before each fix.
2. Compare staff state under its row lock; retain terminal settlement for metadata corrections; validate intake assignees in the insertion transaction.
3. Run formatting, Clippy, workspace tests, and one package review.
4. Deliver one PR with related tickets and merge after CI passes.

No dependencies or API contract changes are needed. Rejected actions preserve records, sessions, and audit history.
