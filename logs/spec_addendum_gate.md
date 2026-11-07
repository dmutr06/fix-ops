# ADD-003 spec gate

Decision: GO.
Scope: preserve one enabled Administrator during staff updates; allow promotion when none remain.
Contract: existing staff PATCH returns `409 CONFLICT` when the last enabled Administrator would be demoted or disabled.
Tests: REQ-F-020, AC-F-020, BDD-021, plus the concurrent demotion case.
