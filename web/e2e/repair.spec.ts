// Browser scenarios for the staff web app (ADD-004): BDD-025 repair in the browser, the work
// queues of BDD-023, sign-out of BDD-022, and only permitted actions offered (REQ-F-021).
// Each step must show its result within 2 seconds (REQ-NF-001).

import { expect, type Page, test } from "@playwright/test";

const STEP_MS = 2_000;
const PASSWORD = "staff-pass-123";

/** Runs `action`, waits for `result`, and checks it took at most 2 seconds. */
async function step(name: string, action: () => Promise<unknown>, result: () => Promise<unknown>) {
    const started = Date.now();
    await action();
    await result();
    const elapsed = Date.now() - started;
    expect(elapsed, `${name} took ${elapsed} ms`).toBeLessThanOrEqual(STEP_MS);
}

async function signIn(page: Page, username: string, password = PASSWORD) {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
    await page.getByLabel("Username").fill(username);
    await page.getByLabel("Password").fill(password);
    await step(
        "sign in",
        () => page.getByRole("button", { name: "Sign in" }).click(),
        () => expect(page.getByRole("heading", { name: "Work queue" })).toBeVisible(),
    );
}

async function signOut(page: Page) {
    await page.getByRole("button", { name: "Sign out" }).click();
    await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
}

async function createStaff(page: Page, username: string, role: string) {
    await page.getByRole("link", { name: "Staff" }).click();
    await page.getByRole("button", { name: "New staff account" }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByLabel("Username").fill(username);
    await dialog.getByLabel("Password").fill(PASSWORD);
    await dialog.getByLabel("Role").selectOption(role);
    await step(
        `create ${role}`,
        () => dialog.getByRole("button", { name: "Create" }).click(),
        () => expect(page.getByRole("cell", { name: username, exact: true })).toBeVisible(),
    );
}

async function openOrderFromQueue(page: Page, section: string, ticket: string) {
    await page.getByRole("link", { name: "Queue" }).click();
    const queue = page.getByRole("region", { name: section });
    await step(
        `open ${ticket} from ${section}`,
        () => queue.getByRole("link", { name: ticket }).click(),
        () => expect(page.getByRole("heading", { name: `Order ${ticket}` })).toBeVisible(),
    );
}

function status(page: Page) {
    return page.getByTestId("order-status");
}

test("BDD-025: three roles take an order from intake to Completed in the browser", async ({
    page,
}, info) => {
    const run = `${info.project.name}-${Date.now()}`;
    const recep = `recep-${run}`;
    const tech = `tech-${run}`;
    const ticket = `WEB-${run}`;
    const client = `Client ${run}`;

    // Administrator: staff accounts.
    await signIn(page, "admin", "admin-pass-123");
    await createStaff(page, recep, "Receptionist");
    await createStaff(page, tech, "Technician");
    await signOut(page);

    // Receptionist: client, device and intake; staff management is not offered.
    await signIn(page, recep);
    await expect(page.getByRole("link", { name: "Staff" })).toHaveCount(0);
    await page.getByRole("link", { name: "Clients" }).click();
    await page.getByRole("button", { name: "New client" }).click();
    let dialog = page.getByRole("dialog");
    await dialog.getByLabel("Name").fill(client);
    await dialog.getByLabel("Phone").fill("+380501112233");
    await step(
        "add client",
        () => dialog.getByRole("button", { name: "Save" }).click(),
        () => expect(page.getByRole("heading", { name: client })).toBeVisible(),
    );
    await page.getByRole("button", { name: "Add device" }).click();
    dialog = page.getByRole("dialog");
    await dialog.getByLabel("Type").selectOption("Laptop");
    await dialog.getByLabel("Serial number").fill(`SN-${run}`);
    await dialog.getByLabel("Reported symptoms").fill("No power");
    await step(
        "add device",
        () => dialog.getByRole("button", { name: "Save" }).click(),
        () => expect(page.getByText(`SN-${run}`)).toBeVisible(),
    );
    await page.getByRole("button", { name: "New order" }).click();
    dialog = page.getByRole("dialog");
    await dialog.getByLabel("Ticket number").fill(ticket);
    await dialog.getByLabel("Technician").selectOption({ label: tech });
    await step(
        "open order",
        () => dialog.getByRole("button", { name: "Create order" }).click(),
        () => expect(page.getByRole("heading", { name: `Order ${ticket}` })).toBeVisible(),
    );
    await expect(status(page)).toHaveText("Received");
    await expect(page.getByRole("button", { name: "Start diagnosis" })).toHaveCount(0);
    await signOut(page);

    // Technician: diagnosis and estimate; cancelling is not offered.
    await signIn(page, tech);
    await openOrderFromQueue(page, "My open orders", ticket);
    await expect(page.getByRole("button", { name: "Cancel order" })).toHaveCount(0);
    await step(
        "start diagnosis",
        () => page.getByRole("button", { name: "Start diagnosis" }).click(),
        () => expect(status(page)).toHaveText("Diagnosing"),
    );
    await page.getByLabel("Diagnostic findings").fill("Broken DC jack");
    await step(
        "save findings",
        () => page.getByRole("button", { name: "Save findings" }).click(),
        () => expect(page.getByTestId("findings")).toHaveText("Broken DC jack"),
    );
    await page.getByLabel("Parts estimate ($)").fill("100.00");
    await page.getByLabel("Labor estimate ($)").fill("50.00");
    await step(
        "save estimate",
        () => page.getByRole("button", { name: "Save estimate" }).click(),
        () => expect(page.getByTestId("estimate")).toHaveText("$100.00 parts + $50.00 labor"),
    );
    await signOut(page);

    // Receptionist: the client approves.
    await signIn(page, recep);
    await openOrderFromQueue(page, "Awaiting client decision", ticket);
    await step(
        "approve",
        () => page.getByRole("button", { name: "Approve" }).click(),
        () => expect(page.getByTestId("decision")).toContainText("Approved"),
    );
    await signOut(page);

    // Technician: repair; Ready is blocked until the repair check.
    await signIn(page, tech);
    await openOrderFromQueue(page, "My open orders", ticket);
    await page.getByLabel("Used parts ($)").fill("100.00");
    await page.getByLabel("Completed labor ($)").fill("50.00");
    await step(
        "start repair",
        () => page.getByRole("button", { name: "Start repair" }).click(),
        () => expect(status(page)).toHaveText("InProgress"),
    );
    await step(
        "ready without check",
        () => page.getByRole("button", { name: "Mark ready" }).click(),
        () => expect(page.getByRole("alert")).toContainText("repair check"),
    );
    await expect(status(page)).toHaveText("InProgress");
    await page.getByLabel("Repair checked").check();
    await step(
        "mark ready",
        () => page.getByRole("button", { name: "Mark ready" }).click(),
        () => expect(status(page)).toHaveText("Ready"),
    );
    await expect(page.getByTestId("settlement")).toHaveText("$150.00");
    await signOut(page);

    // Receptionist: payment, pickup, completion.
    await signIn(page, recep);
    await openOrderFromQueue(page, "Ready for payment and pickup", ticket);
    await expect(page.getByLabel("Payment ($)")).toHaveValue("150.00");
    await step(
        "record payment",
        () => page.getByRole("button", { name: "Record payment" }).click(),
        () => expect(page.getByTestId("payment")).toContainText("$150.00"),
    );
    await step(
        "record pickup",
        () => page.getByRole("button", { name: "Record pickup" }).click(),
        () => expect(page.getByTestId("pickup")).not.toHaveText("—"),
    );
    await step(
        "complete",
        () => page.getByRole("button", { name: "Complete" }).click(),
        () => expect(status(page)).toHaveText("Completed"),
    );
    await expect(page.getByRole("region", { name: "History" })).toContainText(recep);
    await signOut(page);
});

test("BDD-022: signing out ends the browser session", async ({ page }) => {
    await signIn(page, "admin", "admin-pass-123");
    await page.reload();
    await expect(page.getByRole("heading", { name: "Work queue" })).toBeVisible();
    await signOut(page);
    await page.goto("/clients");
    await expect(page.getByRole("heading", { name: "Sign in" })).toBeVisible();
});

test("a wrong password shows the service's message", async ({ page }) => {
    await page.goto("/");
    await page.getByLabel("Username").fill("admin");
    await page.getByLabel("Password").fill("not-the-password");
    await page.getByRole("button", { name: "Sign in" }).click();
    await expect(page.getByRole("alert")).toContainText("Invalid username or password");
});
