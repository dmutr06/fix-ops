import type { ReactNode } from "react";
import { ErrorAlert } from "@/components/common";
import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import { FieldGroup } from "@/components/ui/field";

/** A dialog holding one form; the service's message shows above the buttons on failure. */
export function FormDialog({
    open,
    onOpenChange,
    title,
    submit,
    pending,
    error,
    onSubmit,
    children,
}: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    title: string;
    submit: string;
    pending: boolean;
    error: unknown;
    onSubmit: () => void;
    children: ReactNode;
}) {
    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent>
                <form
                    className="flex flex-col gap-4"
                    onSubmit={(event) => {
                        event.preventDefault();
                        onSubmit();
                    }}
                >
                    <DialogHeader>
                        <DialogTitle>{title}</DialogTitle>
                    </DialogHeader>
                    <FieldGroup>{children}</FieldGroup>
                    <ErrorAlert error={error} />
                    <DialogFooter>
                        <Button type="submit" disabled={pending}>
                            {submit}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    );
}
