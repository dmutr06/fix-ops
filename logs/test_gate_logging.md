# TEST-GATE: Operational logging

**Decision:** GO · **Ticket:** [FIX-53](https://linear.app/fixops/issue/FIX-53) · **Scope:** logging of every request and action outcome; supports REQ-NF-002 (each change identifies one account) and REQ-NF-003 (changes are traceable)

The operational log is separate from the audit log. The audit log is the record of data changes required by REQ-NF-003. The operational log records every request and its outcome, so each test run or manual run can be analysed afterwards.

## Coverage

| Expectation | Test |
|---|---|
| Every request: request id, method, path, status, latency, actor and action once known; `WARN` for 4xx | `every_request_is_logged_with_status_latency_and_actor` |
| Every saved change: actor, record type and id, changed field names | `saved_changes_are_logged_with_record_and_fields` |
| A denied role and a rejected request are logged with the action, role and error code | `denied_and_rejected_actions_are_logged` |
| No password, password hash or session token in the log | `passwords_and_tokens_never_reach_the_log` |

## Baseline run (red)

`cargo test --test logging` with the JSON layer stubbed as a no-op: 0 passed, 4 failed. Every failure is an empty log (`no request line …: []`, `assertion failed: log != ""`), so the tests fail because nothing is logged yet. All other suites stay green.

**Next:** implement FIX-53 without changing these tests.
