# ADD-004 spec gate

Decision: GO (delegated; human review follows).
Scope: staff web interface, browser cookie session, staff list for every role, order work queues, paging ([ADD-004](../spec/addenda/ADD-004-web-interface.md), SRS 1.1.0).
Contract: `openapi.yaml` 1.1.0 adds `getCurrentSession`, `deleteSession`, `cookieAuth`, staff and order filters, `limit`/`offset` and `X-Total-Count`. Existing requests and responses are unchanged.
Tests: REQ-F-021–023, AC-F-021–023, BDD-022–025.
