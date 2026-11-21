import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { ErrorAlert } from "@/components/common";
import { TextField } from "@/components/text-field";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { FieldGroup } from "@/components/ui/field";
import { useSignIn } from "@/lib/session";

const schema = z.object({
    username: z.string().trim().min(1, "Enter your username"),
    password: z.string().min(1, "Enter your password"),
});

export function SignInPage() {
    const signIn = useSignIn();
    const form = useForm({
        resolver: zodResolver(schema),
        defaultValues: { username: "", password: "" },
    });
    const errors = form.formState.errors;
    return (
        <main className="flex min-h-svh items-center justify-center bg-muted/40 p-4">
            <Card className="w-full max-w-sm">
                <CardHeader>
                    <CardTitle>
                        <h1>Sign in</h1>
                    </CardTitle>
                </CardHeader>
                <CardContent>
                    <form
                        className="flex flex-col gap-4"
                        onSubmit={form.handleSubmit((values) => signIn.mutate(values))}
                    >
                        <FieldGroup>
                            <TextField
                                label="Username"
                                autoComplete="username"
                                error={errors.username?.message}
                                {...form.register("username")}
                            />
                            <TextField
                                label="Password"
                                type="password"
                                autoComplete="current-password"
                                error={errors.password?.message}
                                {...form.register("password")}
                            />
                        </FieldGroup>
                        <ErrorAlert error={signIn.error} />
                        <Button type="submit" disabled={signIn.isPending}>
                            Sign in
                        </Button>
                    </form>
                </CardContent>
            </Card>
        </main>
    );
}
