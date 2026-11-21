import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useNavigate, useParams } from "react-router";
import { toast } from "sonner";
import { z } from "zod";
import { Detail, ErrorAlert, OrderList, Section } from "@/components/common";
import { FormDialog } from "@/components/form-dialog";
import { PageSkeleton } from "@/components/motion";
import { SelectField } from "@/components/select-field";
import { TextField } from "@/components/text-field";
import { Button } from "@/components/ui/button";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { api, type Client, call, callPage, type Device } from "@/lib/api";
import { useSession } from "@/lib/session";
import { ClientFields, clientBody, type clientSchema, useClientForm } from "./clients";

const DEVICE_TYPES: Device["device_type"][] = ["Desktop", "Laptop", "Server"];
const TIERS: Client["loyalty_tier"][] = ["Standard", "Silver", "Gold"];

const deviceSchema = z.object({
    device_type: z.enum(DEVICE_TYPES),
    serial_number: z.string().trim().min(1, "Enter the serial number"),
    reported_symptoms: z.string().trim().min(1, "Describe the symptoms"),
});

const orderSchema = z.object({
    ticket_number: z.string().trim().min(1, "Enter a ticket number"),
    assigned_technician_id: z.coerce.number<string>().int().min(1, "Choose a Technician"),
});

function EditClientDialog({ client }: { client: Client }) {
    const queryClient = useQueryClient();
    const [open, setOpen] = useState(false);
    const form = useClientForm(client);
    const save = useMutation({
        mutationFn: (values: z.infer<typeof clientSchema>) =>
            call(
                api.PATCH("/api/v1/clients/{clientId}", {
                    params: { path: { clientId: client.id } },
                    body: { ...clientBody(values), email: values.email || null },
                }),
            ),
        onSuccess: (saved) => {
            queryClient.setQueryData(["client", client.id], saved);
            setOpen(false);
        },
    });
    return (
        <>
            <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
                Edit
            </Button>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title="Edit client"
                submit="Save"
                pending={save.isPending}
                error={save.error}
                onSubmit={form.handleSubmit((values) => save.mutate(values))}
            >
                <ClientFields form={form} />
            </FormDialog>
        </>
    );
}

function TierSelect({ client }: { client: Client }) {
    const queryClient = useQueryClient();
    const save = useMutation({
        mutationFn: (loyalty_tier: Client["loyalty_tier"]) =>
            call(
                api.PUT("/api/v1/clients/{clientId}/loyalty-tier", {
                    params: { path: { clientId: client.id } },
                    body: { loyalty_tier },
                }),
            ),
        onSuccess: (saved) => {
            queryClient.setQueryData(["client", client.id], saved);
            queryClient.invalidateQueries({ queryKey: ["orders"] });
            toast.success(`Loyalty tier set to ${saved.loyalty_tier}`);
        },
        onError: (error) => toast.error(error.message),
    });
    return (
        <NativeSelect
            size="sm"
            aria-label="Loyalty tier"
            value={client.loyalty_tier}
            disabled={save.isPending}
            onChange={(event) => save.mutate(event.target.value as Client["loyalty_tier"])}
        >
            {TIERS.map((tier) => (
                <NativeSelectOption key={tier} value={tier}>
                    {tier}
                </NativeSelectOption>
            ))}
        </NativeSelect>
    );
}

function AddDeviceDialog({ clientId }: { clientId: number }) {
    const queryClient = useQueryClient();
    const [open, setOpen] = useState(false);
    const form = useForm({
        resolver: zodResolver(deviceSchema),
        defaultValues: {
            device_type: "Laptop" as Device["device_type"],
            serial_number: "",
            reported_symptoms: "",
        },
    });
    const create = useMutation({
        mutationFn: (values: z.infer<typeof deviceSchema>) =>
            call(api.POST("/api/v1/devices", { body: { client_id: clientId, ...values } })),
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ["devices", clientId] });
            form.reset();
            setOpen(false);
        },
    });
    const errors = form.formState.errors;
    return (
        <>
            <Button variant="outline" size="sm" onClick={() => setOpen(true)}>
                Add device
            </Button>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title="Add device"
                submit="Save"
                pending={create.isPending}
                error={create.error}
                onSubmit={form.handleSubmit((values) => create.mutate(values))}
            >
                <SelectField
                    label="Type"
                    options={DEVICE_TYPES.map((type) => [type, type])}
                    {...form.register("device_type")}
                />
                <TextField
                    label="Serial number"
                    error={errors.serial_number?.message}
                    {...form.register("serial_number")}
                />
                <TextField
                    label="Reported symptoms"
                    error={errors.reported_symptoms?.message}
                    {...form.register("reported_symptoms")}
                />
            </FormDialog>
        </>
    );
}

function NewOrderDialog({ device }: { device: Device }) {
    const navigate = useNavigate();
    const [open, setOpen] = useState(false);
    const technicians = useQuery({
        queryKey: ["staff", "technicians"],
        queryFn: () =>
            callPage(
                api.GET("/api/v1/staff", {
                    params: { query: { role: "Technician", enabled: true } },
                }),
            ),
        enabled: open,
    });
    const form = useForm({
        resolver: zodResolver(orderSchema),
        defaultValues: { ticket_number: "", assigned_technician_id: "" },
    });
    const create = useMutation({
        mutationFn: (values: z.infer<typeof orderSchema>) =>
            call(
                api.POST("/api/v1/orders", {
                    body: { ...values, client_id: device.client_id, device_id: device.id },
                }),
            ),
        onSuccess: (order) => navigate(`/orders/${order.id}`),
    });
    const errors = form.formState.errors;
    const options: [string, string][] = [
        ["", "Choose a Technician"],
        ...(technicians.data?.items ?? []).map((tech): [string, string] => [
            String(tech.id),
            tech.username,
        ]),
    ];
    return (
        <>
            <Button size="sm" onClick={() => setOpen(true)}>
                New order
            </Button>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title={`New order for ${device.serial_number}`}
                submit="Create order"
                pending={create.isPending}
                error={create.error ?? technicians.error}
                onSubmit={form.handleSubmit((values) => create.mutate(values))}
            >
                <TextField
                    label="Ticket number"
                    error={errors.ticket_number?.message}
                    {...form.register("ticket_number")}
                />
                <SelectField
                    label="Technician"
                    options={options}
                    {...form.register("assigned_technician_id")}
                />
                {errors.assigned_technician_id && (
                    <p className="text-sm text-destructive">
                        {errors.assigned_technician_id.message}
                    </p>
                )}
            </FormDialog>
        </>
    );
}

export function ClientPage() {
    const id = Number(useParams().clientId);
    const { can } = useSession();
    const client = useQuery({
        queryKey: ["client", id],
        queryFn: () =>
            call(api.GET("/api/v1/clients/{clientId}", { params: { path: { clientId: id } } })),
    });
    const devices = useQuery({
        queryKey: ["devices", id],
        queryFn: () =>
            callPage(
                api.GET("/api/v1/devices", { params: { query: { client_id: id, limit: 100 } } }),
            ),
    });
    const orders = useQuery({
        queryKey: ["orders", { client_id: id }],
        queryFn: () =>
            callPage(
                api.GET("/api/v1/orders", { params: { query: { client_id: id, limit: 100 } } }),
            ),
    });
    if (client.isPending) {
        return <PageSkeleton />;
    }
    if (!client.data) {
        return <ErrorAlert error={client.error} />;
    }
    const data = client.data;
    return (
        <>
            <div className="flex flex-wrap items-center justify-between gap-2">
                <h1 className="text-xl font-semibold">{data.name}</h1>
                {can("Receptionist") && <EditClientDialog client={data} />}
            </div>
            <dl className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                <Detail label="Phone">{data.phone}</Detail>
                <Detail label="Email">{data.email ?? "—"}</Detail>
                <Detail label="Loyalty tier">
                    {can() ? <TierSelect client={data} /> : data.loyalty_tier}
                </Detail>
            </dl>
            <Section
                title="Devices"
                actions={can("Receptionist") && <AddDeviceDialog clientId={id} />}
            >
                {devices.data?.items.length ? (
                    <ul className="divide-y rounded-lg border">
                        {devices.data.items.map((device) => (
                            <li
                                key={device.id}
                                className="flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-2"
                            >
                                <span className="font-medium">{device.serial_number}</span>
                                <span className="text-sm text-muted-foreground">
                                    {device.device_type} · {device.reported_symptoms}
                                </span>
                                {can("Receptionist") && (
                                    <span className="ml-auto">
                                        <NewOrderDialog device={device} />
                                    </span>
                                )}
                            </li>
                        ))}
                    </ul>
                ) : (
                    <p className="text-sm text-muted-foreground">No devices yet.</p>
                )}
                <ErrorAlert error={devices.error} />
            </Section>
            <Section title="Orders">
                <OrderList orders={orders.data?.items ?? []} empty="No orders yet." />
            </Section>
        </>
    );
}
