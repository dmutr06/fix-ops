import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { AnimatePresence, motion } from "motion/react";
import { type ReactNode, useId, useState } from "react";
import { Link, useParams } from "react-router";
import { Detail, ErrorAlert, Section, StatusBadge } from "@/components/common";
import { FormDialog } from "@/components/form-dialog";
import { FadeIn, listItem, PageSkeleton } from "@/components/motion";
import { SelectField } from "@/components/select-field";
import { TextField } from "@/components/text-field";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldLabel } from "@/components/ui/field";
import { Textarea } from "@/components/ui/textarea";
import { ApiError, api, call, callPage, type Order, type Staff } from "@/lib/api";
import { cents, dateTime, dollars, money } from "@/lib/format";
import { useSession } from "@/lib/session";

type Path = { params: { path: { orderId: number } } };

/** Runs order commands; each success replaces the cached order, a failure shows its message. */
function useOrderCommand(orderId: number) {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (command: (path: Path) => Promise<Order>) =>
            command({ params: { path: { orderId } } }),
        onSuccess: (order) => {
            queryClient.setQueryData(["order", orderId], order);
            queryClient.invalidateQueries({ queryKey: ["history", orderId] });
            queryClient.invalidateQueries({ queryKey: ["orders"] });
        },
    });
}

type Command = ReturnType<typeof useOrderCommand>;

function amount(text: string, label: string): number {
    const value = cents(text);
    if (value === null) {
        throw new ApiError(400, `${label} must be an amount like 12.50`);
    }
    return value;
}

function MoneyField({
    label,
    value,
    onChange,
}: {
    label: string;
    value: string;
    onChange: (value: string) => void;
}) {
    return (
        <TextField
            label={label}
            inputMode="decimal"
            value={value}
            onChange={(event) => onChange(event.target.value)}
        />
    );
}

function Row({ children }: { children: ReactNode }) {
    return <div className="grid gap-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end">{children}</div>;
}

const now = () => new Date().toISOString();

/** The steps the signed-in role may take on the order in its current state. */
function Actions({ order, command }: { order: Order; command: Command }) {
    const { can } = useSession();
    const bench = can("Technician");
    const front = can("Receptionist");
    const [findings, setFindings] = useState(order.diagnostic_findings ?? "");
    const [parts, setParts] = useState(order.estimate ? dollars(order.estimate.parts_cents) : "");
    const [labor, setLabor] = useState(order.estimate ? dollars(order.estimate.labor_cents) : "");
    const [usedParts, setUsedParts] = useState(dollars(order.used_parts_cents));
    const [doneLabor, setDoneLabor] = useState(dollars(order.completed_labor_cents));
    const [checked, setChecked] = useState(order.repair_checked);
    const [payment, setPayment] = useState(dollars(order.settlement_cents));
    const findingsId = useId();
    const checkId = useId();
    const run = command.mutate;
    const pending = command.isPending;
    const decided = order.client_decision !== null;
    const approved = order.client_decision?.decision === "Approved";
    const open = ["Received", "Diagnosing", "InProgress"].includes(order.status);
    const repair = (status: "InProgress" | "Ready", withCheck: boolean) =>
        run((path) =>
            call(
                api.PUT("/api/v1/orders/{orderId}/repair", {
                    ...path,
                    body: {
                        status,
                        used_parts_cents: amount(usedParts, "Used parts"),
                        completed_labor_cents: amount(doneLabor, "Completed labor"),
                        ...(withCheck ? { repair_checked: checked } : {}),
                    },
                }),
            ),
        );

    const steps: ReactNode[] = [];
    if (bench && order.status === "Received") {
        steps.push(
            <Button
                key="start"
                disabled={pending}
                onClick={() =>
                    run((path) =>
                        call(api.PUT("/api/v1/orders/{orderId}/diagnosis", { ...path, body: {} })),
                    )
                }
            >
                Start diagnosis
            </Button>,
        );
    }
    if (bench && order.status === "Diagnosing" && !decided) {
        steps.push(
            <div key="findings" className="flex flex-col gap-3">
                <Field>
                    <FieldLabel htmlFor={findingsId}>Diagnostic findings</FieldLabel>
                    <Textarea
                        id={findingsId}
                        value={findings}
                        onChange={(event) => setFindings(event.target.value)}
                    />
                </Field>
                <Button
                    className="self-start"
                    disabled={pending}
                    onClick={() =>
                        run((path) =>
                            call(
                                api.PUT("/api/v1/orders/{orderId}/diagnosis", {
                                    ...path,
                                    body: { diagnostic_findings: findings },
                                }),
                            ),
                        )
                    }
                >
                    Save findings
                </Button>
            </div>,
            <Row key="estimate">
                <MoneyField label="Parts estimate ($)" value={parts} onChange={setParts} />
                <MoneyField label="Labor estimate ($)" value={labor} onChange={setLabor} />
                <Button
                    disabled={pending}
                    onClick={() =>
                        run((path) =>
                            call(
                                api.PUT("/api/v1/orders/{orderId}/estimate", {
                                    ...path,
                                    body: {
                                        parts_cents: amount(parts, "Parts estimate"),
                                        labor_cents: amount(labor, "Labor estimate"),
                                    },
                                }),
                            ),
                        )
                    }
                >
                    Save estimate
                </Button>
            </Row>,
        );
    }
    if (front && order.status === "Diagnosing" && order.estimate && !decided) {
        const decide = (decision: "Approved" | "Declined") =>
            run((path) =>
                call(
                    api.PUT("/api/v1/orders/{orderId}/decision", {
                        ...path,
                        body: { decision, decided_at: now() },
                    }),
                ),
            );
        steps.push(
            <div key="decision" className="flex flex-wrap gap-2">
                <Button disabled={pending} onClick={() => decide("Approved")}>
                    Approve
                </Button>
                <Button variant="outline" disabled={pending} onClick={() => decide("Declined")}>
                    Decline
                </Button>
            </div>,
        );
    }
    if (bench && order.status === "Diagnosing" && approved) {
        steps.push(
            <Row key="start-repair">
                <MoneyField label="Used parts ($)" value={usedParts} onChange={setUsedParts} />
                <MoneyField label="Completed labor ($)" value={doneLabor} onChange={setDoneLabor} />
                <Button disabled={pending} onClick={() => repair("InProgress", false)}>
                    Start repair
                </Button>
            </Row>,
        );
    }
    if (bench && order.status === "InProgress") {
        steps.push(
            <div key="repair" className="flex flex-col gap-3">
                <Row>
                    <MoneyField label="Used parts ($)" value={usedParts} onChange={setUsedParts} />
                    <MoneyField
                        label="Completed labor ($)"
                        value={doneLabor}
                        onChange={setDoneLabor}
                    />
                    <Button
                        variant="outline"
                        disabled={pending}
                        onClick={() => repair("InProgress", true)}
                    >
                        Save
                    </Button>
                </Row>
                <div className="flex flex-wrap items-center gap-4">
                    <label htmlFor={checkId} className="flex items-center gap-2 text-sm">
                        <input
                            id={checkId}
                            type="checkbox"
                            className="size-4 accent-primary"
                            checked={checked}
                            onChange={(event) => setChecked(event.target.checked)}
                        />
                        Repair checked
                    </label>
                    <Button disabled={pending} onClick={() => repair("Ready", true)}>
                        Mark ready
                    </Button>
                </div>
            </div>,
        );
    }
    if (front && order.status === "Ready") {
        steps.push(
            <Row key="payment">
                <MoneyField label="Payment ($)" value={payment} onChange={setPayment} />
                <span />
                <Button
                    disabled={pending}
                    onClick={() =>
                        run((path) =>
                            call(
                                api.PUT("/api/v1/orders/{orderId}/payment", {
                                    ...path,
                                    body: {
                                        amount_cents: amount(payment, "Payment"),
                                        paid_at: now(),
                                    },
                                }),
                            ),
                        )
                    }
                >
                    Record payment
                </Button>
            </Row>,
            <div key="finish" className="flex flex-wrap gap-2">
                <Button
                    variant="outline"
                    disabled={pending}
                    onClick={() =>
                        run((path) =>
                            call(
                                api.PUT("/api/v1/orders/{orderId}/pickup", {
                                    ...path,
                                    body: { picked_up_at: now() },
                                }),
                            ),
                        )
                    }
                >
                    Record pickup
                </Button>
                <Button
                    disabled={pending}
                    onClick={() =>
                        run((path) =>
                            call(api.POST("/api/v1/orders/{orderId}/complete", { ...path })),
                        )
                    }
                >
                    Complete
                </Button>
            </div>,
        );
    }
    if (front && open) {
        steps.push(<CancelButton key="cancel" command={command} />);
    }
    if (steps.length === 0) {
        return null;
    }
    return (
        <Section title="Actions">
            <motion.div layout transition={{ duration: 0.2 }} className="rounded-lg border p-4">
                <FadeIn layout="position" className="flex flex-col gap-4">
                    {steps}
                </FadeIn>
            </motion.div>
            <AnimatePresence>
                {command.error && (
                    <FadeIn key="error" exit={{ opacity: 0 }}>
                        <ErrorAlert error={command.error} />
                    </FadeIn>
                )}
            </AnimatePresence>
        </Section>
    );
}

function CancelButton({ command }: { command: Command }) {
    const [open, setOpen] = useState(false);
    return (
        <>
            <Button variant="destructive" className="self-start" onClick={() => setOpen(true)}>
                Cancel order
            </Button>
            <Dialog open={open} onOpenChange={setOpen}>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>Cancel this order?</DialogTitle>
                        <DialogDescription>
                            The cancellation charge depends on how far the order got.
                        </DialogDescription>
                    </DialogHeader>
                    <DialogFooter>
                        <Button
                            variant="destructive"
                            disabled={command.isPending}
                            onClick={() => {
                                setOpen(false);
                                command.mutate((path) =>
                                    call(api.POST("/api/v1/orders/{orderId}/cancel", { ...path })),
                                );
                            }}
                        >
                            Cancel order
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </>
    );
}

/** Administrator corrections (REQ-F-018); the service still applies every rule. */
function CorrectDialog({ order, technicians }: { order: Order; technicians: Staff[] }) {
    const [open, setOpen] = useState(false);
    const command = useOrderCommand(order.id);
    const [ticket, setTicket] = useState(order.ticket_number);
    const [tech, setTech] = useState(String(order.assigned_technician_id));
    const [usedParts, setUsedParts] = useState(dollars(order.used_parts_cents));
    const [labor, setLabor] = useState(dollars(order.completed_labor_cents));
    const submit = () =>
        command.mutate(
            (path) => {
                const body: Record<string, unknown> = {};
                if (ticket !== order.ticket_number) body.ticket_number = ticket;
                if (Number(tech) !== order.assigned_technician_id) {
                    body.assigned_technician_id = Number(tech);
                }
                const parts = amount(usedParts, "Used parts");
                if (parts !== order.used_parts_cents) body.used_parts_cents = parts;
                const done = amount(labor, "Completed labor");
                if (done !== order.completed_labor_cents) body.completed_labor_cents = done;
                return call(api.PATCH("/api/v1/orders/{orderId}", { ...path, body }));
            },
            { onSuccess: () => setOpen(false) },
        );
    return (
        <>
            <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
                Correct
            </Button>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title="Correct order"
                submit="Save correction"
                pending={command.isPending}
                error={command.error}
                onSubmit={submit}
            >
                <TextField
                    label="Ticket number"
                    value={ticket}
                    onChange={(event) => setTicket(event.target.value)}
                />
                <SelectField
                    label="Technician"
                    value={tech}
                    onChange={(event) => setTech(event.target.value)}
                    options={technicians.map((staff) => [String(staff.id), staff.username])}
                />
                <MoneyField label="Used parts ($)" value={usedParts} onChange={setUsedParts} />
                <MoneyField label="Completed labor ($)" value={labor} onChange={setLabor} />
            </FormDialog>
        </>
    );
}

function History({ orderId, names }: { orderId: number; names: Map<number, string> }) {
    const history = useQuery({
        queryKey: ["history", orderId],
        queryFn: () =>
            callPage(
                api.GET("/api/v1/audit-logs", {
                    params: {
                        query: { record_type: "RepairOrder", record_id: orderId, limit: 100 },
                    },
                }),
            ),
    });
    return (
        <Section title="History">
            <ul className="flex flex-col gap-1 text-sm">
                <AnimatePresence initial={false}>
                    {history.data?.items.map((entry) => (
                        <motion.li key={entry.id} {...listItem} className="flex flex-wrap gap-x-2">
                            <span className="text-muted-foreground">
                                {dateTime(entry.recorded_at)}
                            </span>
                            <span className="font-medium">
                                {names.get(entry.actor_id) ?? `#${entry.actor_id}`}
                            </span>
                            <span>
                                {entry.field}: {entry.previous_value ?? "—"} →{" "}
                                {entry.new_value ?? "—"}
                            </span>
                        </motion.li>
                    ))}
                </AnimatePresence>
            </ul>
            <ErrorAlert error={history.error} />
        </Section>
    );
}

export function OrderPage() {
    const id = Number(useParams().orderId);
    const { can } = useSession();
    const command = useOrderCommand(id);
    const order = useQuery({
        queryKey: ["order", id],
        queryFn: () =>
            call(api.GET("/api/v1/orders/{orderId}", { params: { path: { orderId: id } } })),
    });
    const staff = useQuery({
        queryKey: ["staff"],
        queryFn: () => callPage(api.GET("/api/v1/staff", { params: { query: { limit: 100 } } })),
    });
    const clientId = order.data?.client_id;
    const deviceId = order.data?.device_id;
    const client = useQuery({
        queryKey: ["client", clientId],
        queryFn: () =>
            call(
                api.GET("/api/v1/clients/{clientId}", {
                    params: { path: { clientId: clientId ?? 0 } },
                }),
            ),
        enabled: clientId !== undefined,
    });
    const device = useQuery({
        queryKey: ["device", deviceId],
        queryFn: () =>
            call(
                api.GET("/api/v1/devices/{deviceId}", {
                    params: { path: { deviceId: deviceId ?? 0 } },
                }),
            ),
        enabled: deviceId !== undefined,
    });
    if (order.isPending) {
        return <PageSkeleton />;
    }
    if (!order.data) {
        return <ErrorAlert error={order.error} />;
    }
    const data = order.data;
    const names = new Map((staff.data?.items ?? []).map((s) => [s.id, s.username]));
    const technicians = (staff.data?.items ?? []).filter((s) => s.role === "Technician");
    return (
        <>
            <div className="flex flex-wrap items-center gap-3">
                <h1 className="text-xl font-semibold">Order {data.ticket_number}</h1>
                <StatusBadge status={data.status} testId="order-status" />
                {can() && (
                    <span className="ml-auto">
                        <CorrectDialog
                            key={JSON.stringify(data)}
                            order={data}
                            technicians={technicians}
                        />
                    </span>
                )}
            </div>
            <dl className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                <Detail label="Client">
                    {client.data ? (
                        <Link
                            to={`/clients/${data.client_id}`}
                            className="underline-offset-4 hover:underline"
                        >
                            {client.data.name}
                        </Link>
                    ) : (
                        "…"
                    )}
                </Detail>
                <Detail label="Device">
                    {device.data ? `${device.data.device_type} ${device.data.serial_number}` : "…"}
                </Detail>
                <Detail label="Technician">{names.get(data.assigned_technician_id) ?? "…"}</Detail>
                <Detail label="Findings" testId="findings">
                    {data.diagnostic_findings ?? "—"}
                </Detail>
                <Detail label="Estimate" testId="estimate">
                    {data.estimate
                        ? `${money(data.estimate.parts_cents)} parts + ${money(data.estimate.labor_cents)} labor`
                        : "—"}
                </Detail>
                <Detail label="Client decision" testId="decision">
                    {data.client_decision
                        ? `${data.client_decision.decision} · ${dateTime(data.client_decision.decided_at)}`
                        : "—"}
                </Detail>
                <Detail label="Used parts">{money(data.used_parts_cents)}</Detail>
                <Detail label="Completed labor">{money(data.completed_labor_cents)}</Detail>
                <Detail label="Repair checked">{data.repair_checked ? "Yes" : "No"}</Detail>
                <Detail label="Loyalty discount">{money(data.labor_discount_cents)}</Detail>
                <Detail label="Settlement" testId="settlement">
                    {money(data.settlement_cents)}
                </Detail>
                <Detail label="Payment" testId="payment">
                    {data.payment
                        ? `${money(data.payment.amount_cents)} · ${dateTime(data.payment.paid_at)}`
                        : "—"}
                </Detail>
                <Detail label="Pickup" testId="pickup">
                    {data.pickup ? dateTime(data.pickup.picked_up_at) : "—"}
                </Detail>
            </dl>
            <Actions
                key={`${data.status}-${data.settlement_cents}`}
                order={data}
                command={command}
            />
            <History orderId={id} names={names} />
        </>
    );
}
