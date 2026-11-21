# Concept: FixOps staff web app

## Purpose
The browser interface that [SRS 1.0.3](srs.md) §2 promises and REQ-NF-001 times. Receptionists, Technicians and Administrators do their daily work in it instead of calling the API. Rules, pricing and audit stay in the service; the web app only shows and submits them.

## Users and screens
- **Everyone:** sign in, sign out, search clients, devices and orders, open an order with its history.
- **Receptionist:** new client and device, new repair order (find or add the client → device → Technician → ticket number), client decision, payment, pickup, complete, cancel.
- **Technician:** my assigned orders, diagnosis and findings, estimate, used parts, labor, repair check, mark Ready.
- **Administrator:** everything above, staff accounts, loyalty tiers, order corrections.

After sign-in each role lands on its work queue: the Receptionist sees orders waiting for a client decision, payment or pickup; the Technician sees their assigned open orders; the Administrator sees all open orders by status.

An order screen shows its status, the next allowed actions for the signed-in role, and the settlement in dollars. A blocked step says which guard is missing.

## Stack
- React, TypeScript and Vite in `web/`. The build is embedded in the service binary and served at `/`; the API stays at `/api/v1` on the same origin.
- English labels from the [glossary](glossary.md); amounts in USD with two decimals.
- Works on phone, tablet and desktop, from 360px wide.
- [shadcn/ui](https://ui.shadcn.com/docs/installation/vite) with Base UI primitives on Tailwind CSS v4 (`@tailwindcss/vite`), CSS-variable theme with light and dark mode.
- Forms: shadcn `Field` with React Hook Form and Zod. Tables: shadcn data table on TanStack Table. Toasts: Sonner.
- API types generated from [`openapi.yaml`](openapi.yaml); data with TanStack Query.
- Browser tests with Playwright, including REQ-NF-001 timing.

## Known API gaps
1. A Receptionist cannot list Technicians (`listStaffAccounts` is Administrator-only), so intake cannot offer a choice.
2. Orders cannot be filtered by assigned Technician or by the step they wait for, so the work queues cannot be built.
3. List operations return every record; there is no paging for 10,000 orders.
4. The audit log names actors by `actor_id` only.
5. Sessions use a bearer token, which scripts in the browser could read. The web app signs in with a Secure, HttpOnly, SameSite=Strict cookie instead; bearer tokens stay for API clients.

One addendum (ADD-004) covers all five, with contract tests and backend changes, before any screen uses them.

## Out of scope
Customer portal, notifications, native mobile apps, offline use, translations, and any change to business rules.
