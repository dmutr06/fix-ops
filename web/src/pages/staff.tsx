import { zodResolver } from "@hookform/resolvers/zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { toast } from "sonner";
import { z } from "zod";
import { ErrorAlert, Loading, Section } from "@/components/common";
import { FormDialog } from "@/components/form-dialog";
import { SelectField } from "@/components/select-field";
import { TextField } from "@/components/text-field";
import { Button } from "@/components/ui/button";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { api, call, callPage, type Role, type Staff } from "@/lib/api";

const ROLES: Role[] = ["Receptionist", "Technician", "Administrator"];

const schema = z.object({
    username: z.string().trim().min(1, "Enter a username"),
    password: z.string().min(8, "At least 8 characters"),
    role: z.enum(ROLES),
});

function NewStaffDialog() {
    const queryClient = useQueryClient();
    const [open, setOpen] = useState(false);
    const form = useForm({
        resolver: zodResolver(schema),
        defaultValues: { username: "", password: "", role: "Receptionist" as Role },
    });
    const create = useMutation({
        mutationFn: (body: z.infer<typeof schema>) => call(api.POST("/api/v1/staff", { body })),
        onSuccess: (staff) => {
            queryClient.invalidateQueries({ queryKey: ["staff"] });
            toast.success(`Created ${staff.username}`);
            form.reset();
            setOpen(false);
        },
    });
    const errors = form.formState.errors;
    return (
        <>
            <Button size="sm" onClick={() => setOpen(true)}>
                New staff account
            </Button>
            <FormDialog
                open={open}
                onOpenChange={setOpen}
                title="New staff account"
                submit="Create"
                pending={create.isPending}
                error={create.error}
                onSubmit={form.handleSubmit((values) => create.mutate(values))}
            >
                <TextField
                    label="Username"
                    error={errors.username?.message}
                    {...form.register("username")}
                />
                <TextField
                    label="Password"
                    type="password"
                    autoComplete="new-password"
                    error={errors.password?.message}
                    {...form.register("password")}
                />
                <SelectField
                    label="Role"
                    options={ROLES.map((role) => [role, role])}
                    {...form.register("role")}
                />
            </FormDialog>
        </>
    );
}

function StaffRow({ account }: { account: Staff }) {
    const queryClient = useQueryClient();
    const update = useMutation({
        mutationFn: (body: { role?: Role; enabled?: boolean }) =>
            call(
                api.PATCH("/api/v1/staff/{staffId}", {
                    params: { path: { staffId: account.id } },
                    body,
                }),
            ),
        onSuccess: () => queryClient.invalidateQueries({ queryKey: ["staff"] }),
        onError: (error) => toast.error(error.message),
    });
    return (
        <tr className="border-t">
            <td className="px-3 py-2">{account.username}</td>
            <td className="px-3 py-2">
                <NativeSelect
                    size="sm"
                    aria-label={`Role of ${account.username}`}
                    value={account.role}
                    disabled={update.isPending}
                    onChange={(event) => update.mutate({ role: event.target.value as Role })}
                >
                    {ROLES.map((role) => (
                        <NativeSelectOption key={role} value={role}>
                            {role}
                        </NativeSelectOption>
                    ))}
                </NativeSelect>
            </td>
            <td className="px-3 py-2 text-right">
                <Button
                    variant="outline"
                    size="sm"
                    disabled={update.isPending}
                    onClick={() => update.mutate({ enabled: !account.enabled })}
                >
                    {account.enabled ? "Disable" : "Enable"}
                </Button>
            </td>
        </tr>
    );
}

/** Staff accounts, for Administrators (REQ-F-019, REQ-F-020). */
export function StaffPage() {
    const staff = useQuery({
        queryKey: ["staff"],
        queryFn: () => callPage(api.GET("/api/v1/staff", { params: { query: { limit: 100 } } })),
    });
    return (
        <>
            <h1 className="text-xl font-semibold">Staff</h1>
            <Section title="Accounts" actions={<NewStaffDialog />}>
                {staff.isPending ? (
                    <Loading />
                ) : (
                    <div className="overflow-x-auto rounded-lg border">
                        <table className="w-full text-sm">
                            <thead className="text-left text-muted-foreground">
                                <tr>
                                    <th className="px-3 py-2 font-medium">Username</th>
                                    <th className="px-3 py-2 font-medium">Role</th>
                                    <th className="px-3 py-2" />
                                </tr>
                            </thead>
                            <tbody>
                                {staff.data?.items.map((account) => (
                                    <StaffRow key={account.id} account={account} />
                                ))}
                            </tbody>
                        </table>
                    </div>
                )}
                <ErrorAlert error={staff.error} />
            </Section>
        </>
    );
}
