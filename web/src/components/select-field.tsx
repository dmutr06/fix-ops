import { type ComponentProps, useId } from "react";
import { Field, FieldLabel } from "@/components/ui/field";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";

/** A labelled native select over `options` (value, label). */
export function SelectField({
    label,
    options,
    ...props
}: Omit<ComponentProps<"select">, "size"> & { label: string; options: [string, string][] }) {
    const id = useId();
    return (
        <Field>
            <FieldLabel htmlFor={id}>{label}</FieldLabel>
            <NativeSelect id={id} className="w-full" {...props}>
                {options.map(([value, text]) => (
                    <NativeSelectOption key={value} value={value}>
                        {text}
                    </NativeSelectOption>
                ))}
            </NativeSelect>
        </Field>
    );
}
