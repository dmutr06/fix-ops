import { zodResolver } from "@hookform/resolvers/zod";
import { keepPreviousData, useMutation, useQuery } from "@tanstack/react-query";
import { AnimatePresence, motion } from "motion/react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { Link, useNavigate } from "react-router";
import { z } from "zod";
import { ErrorAlert, PAGE_SIZE, Pager } from "@/components/common";
import { FormDialog } from "@/components/form-dialog";
import { ListSkeleton, listItem } from "@/components/motion";
import { TextField } from "@/components/text-field";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { api, type Client, call, callPage } from "@/lib/api";
import { useSession } from "@/lib/session";

export const clientSchema = z.object({
    name: z.string().trim().min(1, "Enter a name"),
    phone: z.string().trim().min(1, "Enter a phone number"),
    email: z.union([z.literal(""), z.email("Enter a valid email")]),
});

/** Client fields of the contract from the form: an empty email is left out. */
export function clientBody(values: z.infer<typeof clientSchema>) {
    return {
        name: values.name,
        phone: values.phone,
        ...(values.email ? { email: values.email } : {}),
    };
}

export function ClientFields({ form }: { form: ReturnType<typeof useClientForm> }) {
    const errors = form.formState.errors;
    return (
        <>
            <TextField label="Name" error={errors.name?.message} {...form.register("name")} />
            <TextField
                label="Phone"
                type="tel"
                error={errors.phone?.message}
                {...form.register("phone")}
            />
            <TextField
                label="Email"
                type="email"
                error={errors.email?.message}
                {...form.register("email")}
            />
        </>
    );
}

export function useClientForm(client?: Client) {
    return useForm({
        resolver: zodResolver(clientSchema),
        defaultValues: {
            name: client?.name ?? "",
            phone: client?.phone ?? "",
            email: client?.email ?? "",
        },
    });
}

function NewClientDialog() {
    const navigate = useNavigate();
    const [open, setOpen] = useState(false);
    const form = useClientForm();
    const create = useMutation({
        mutationFn: (values: z.infer<typeof clientSchema>) =>
            call(api.POST("/api/v1/clients", { body: clientBody(values) })),
        onSuccess: (client) => navigate(`/clients/${client.id}`),
    });
    return (
        <>
            <Button size="sm" onClick={() => setOpen(true)}>
                New client
            </Button>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title="New client"
                submit="Save"
                pending={create.isPending}
                error={create.error}
                onSubmit={form.handleSubmit((values) => create.mutate(values))}
            >
                <ClientFields form={form} />
            </FormDialog>
        </>
    );
}

export function ClientsPage() {
    const { can } = useSession();
    const [text, setText] = useState("");
    const [offset, setOffset] = useState(0);
    const query = { q: text.trim() || undefined, limit: PAGE_SIZE, offset };
    const clients = useQuery({
        queryKey: ["clients", query],
        placeholderData: keepPreviousData,
        queryFn: () => callPage(api.GET("/api/v1/clients", { params: { query } })),
    });
    return (
        <>
            <div className="flex flex-wrap items-center justify-between gap-2">
                <h1 className="text-xl font-semibold">Clients</h1>
                {can("Receptionist") && <NewClientDialog />}
            </div>
            <Input
                aria-label="Search clients"
                placeholder="Search by name or phone"
                value={text}
                onChange={(event) => {
                    setText(event.target.value);
                    setOffset(0);
                }}
            />
            {clients.isPending ? (
                <ListSkeleton />
            ) : clients.data?.items.length ? (
                <ul className="divide-y overflow-hidden rounded-lg border">
                    <AnimatePresence initial={false}>
                        {clients.data.items.map((client) => (
                            <motion.li
                                key={client.id}
                                {...listItem}
                                className="flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-2"
                            >
                                <Link
                                    to={`/clients/${client.id}`}
                                    className="font-medium underline-offset-4 hover:underline"
                                >
                                    {client.name}
                                </Link>
                                <span className="text-sm text-muted-foreground">
                                    {client.phone}
                                </span>
                                <span className="ml-auto text-sm text-muted-foreground">
                                    {client.loyalty_tier}
                                </span>
                            </motion.li>
                        ))}
                    </AnimatePresence>
                </ul>
            ) : (
                <p className="text-sm text-muted-foreground">No clients match.</p>
            )}
            <Pager page={clients.data} offset={offset} onOffset={setOffset} />
            <ErrorAlert error={clients.error} />
        </>
    );
}
