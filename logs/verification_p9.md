# P9 verification

Decision: GO (delegated; human review follows).
Scope: staff web app for REQ-F-021, REQ-F-022, REQ-F-023 and browser timing of REQ-NF-001 ([ADD-004](../spec/addenda/ADD-004-web-interface.md)).
Checks: Biome and TypeScript clean; Playwright 6/6 on desktop and 360px phone against the real service and a fresh database; every BDD-025 step within 2 s locally; `cargo test --workspace` 232 passed; rustdoc and clippy clean.
Review: the app calls only the public API, with types generated from `openapi.yaml`, so it adds no rule. The session lives in the HttpOnly cookie; a `401` from any request returns to sign-in. Each role sees only its actions, and blocked steps show the service's message. Screenshots on desktop and phone checked by eye.
Limits: the 2-second check runs on an almost empty database; the 10,000-order API timing stays with `scripts/check-performance.py`. The human walkthrough of the web app is part of WEB-RELEASE-GATE.
