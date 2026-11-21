# Concept: FixOps staff web app

## Purpose
The browser interface that [SRS 1.0.3](srs.md) §2 promises and REQ-NF-001 times. Receptionists, Technicians and Administrators do their daily work in it instead of calling the API. Rules, pricing and audit stay in the service; the web app only shows and submits them.

## Users and screens
- **Everyone:** sign in, sign out, search clients, devices and orders, open an order with its history.
- **Receptionist:** new client and device, new repair order (find or add the client → device → Technician → ticket number), client decision, payment, pickup, complete, cancel.
- **Technician:** my assigned orders, diagnosis and findings, estimate, used parts, labor, repair check, mark Ready.
- **Administrator:** everything above, staff accounts, loyalty tiers, order corrections.

An order screen shows its status, the next allowed actions for the signed-in role, and the settlement in dollars. A blocked step says which guard is missing.

## Stack
- React, TypeScript and Vite in `web/`, served by the service at `/`; the API stays at `/api/v1`.
- [shadcn/ui](https://ui.shadcn.com/docs/installation/vite) on Tailwind CSS v4 (`@tailwindcss/vite`), CSS-variable theme with light and dark mode.
- Forms: shadcn `Field` with React Hook Form and Zod. Tables: shadcn data table on TanStack Table. Toasts: Sonner.
- API types generated from [`openapi.yaml`](openapi.yaml); data with TanStack Query.
- Browser tests with Playwright, including REQ-NF-001 timing.

## Known API gaps
1. A Receptionist cannot list Technicians (`listStaffAccounts` is Administrator-only), so intake cannot offer a choice.
2. Orders cannot be filtered by assigned Technician, so there is no "my orders" list.
3. List operations return every record; there is no paging for 10,000 orders.
4. The audit log names actors by `actor_id` only.
5. The bearer token must live in the browser.

These need an SRS addendum and contract changes before the screens that use them.

## Out of scope
Customer portal, notifications, mobile app, offline use, translations, and any rule change.
