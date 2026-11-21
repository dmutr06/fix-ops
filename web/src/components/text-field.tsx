import { type ComponentProps, useId } from "react";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";

/** A labelled input with its validation message. */
export function TextField({
    label,
    error,
    ...props
}: ComponentProps<typeof Input> & { label: string; error?: string }) {
    const id = useId();
    return (
        <Field data-invalid={error ? true : undefined}>
            <FieldLabel htmlFor={id}>{label}</FieldLabel>
            <Input id={id} aria-invalid={error ? true : undefined} {...props} />
            <FieldError>{error}</FieldError>
        </Field>
    );
}
