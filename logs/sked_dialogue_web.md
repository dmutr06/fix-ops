# Specification discovery summary: staff web app

Source: [`concept-web.md`](../spec/concept-web.md), following [FINAL-GATE](final_gate.md). Two dialogue rounds, seven decisions, none deferred.

## Decisions

1. **API gaps:** one addendum, ADD-004, closes all of them before any screen uses them: Technician list for Receptionists, queue filters, paging, actor names, cookie sessions.
2. **Session:** Secure, HttpOnly, SameSite=Strict cookie for the web app; bearer tokens stay for API clients.
3. **Components:** shadcn/ui with Base UI primitives on Tailwind CSS v4.
4. **Language:** English labels from the glossary; USD with two decimals.
5. **Devices:** phone, tablet and desktop, from 360px wide.
6. **Home:** a work queue per role.
7. **Deployment:** the web build is embedded in the service binary and served at `/`, same origin as the API.

## Unchanged

Business rules, pricing, roles and audit stay as in SRS 1.0.3; the web app adds no rule of its own.
