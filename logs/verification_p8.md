# P8 verification

Decision: GO (delegated; human review follows).
Scope: ADD-004 API changes for REQ-F-001, REQ-F-002, REQ-F-022, REQ-F-023, REQ-NF-002.
Checks: fmt, clippy with warnings denied, rustdoc, and the HTTP tests pass; `web_api.rs` covers BDD-022–024 and paging on all five lists.
Review: the cookie is `HttpOnly; Secure; SameSite=Strict`, and bearer tokens take precedence over it. Sign-out deletes the stored session, so a copied token stops working too. Filters and paging run in SQL, with a separate count query. Account changes stay Administrator-only; two older tests used the staff list as their denial probe and now use Administrator-only actions.
Note: order lists still read each order's client to compute its settlement; with `limit` at most 100 this stays bounded.
