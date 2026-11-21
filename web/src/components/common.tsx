import { type ReactNode, useId } from "react";
import { Link } from "react-router";
import { enter } from "@/components/loading";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import type { Order, Page } from "@/lib/api";
import { money } from "@/lib/format";

/** A titled page region, so assistive technology and tests can find it by its heading. */
export function Section({
    title,
    actions,
    children,
}: {
    title: string;
    actions?: ReactNode;
    children: ReactNode;
}) {
    const id = useId();
    return (
        <section aria-labelledby={id} className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
                <h2 id={id} className="text-base font-semibold">
                    {title}
                </h2>
                {actions}
            </div>
            {children}
        </section>
    );
}

export function ErrorAlert({ error }: { error: unknown }) {
    if (!error) {
        return null;
    }
    return (
        <Alert variant="destructive">
            <AlertDescription>
                {error instanceof Error ? error.message : "Something went wrong"}
            </AlertDescription>
        </Alert>
    );
}

const statusVariant: Record<Order["status"], "default" | "secondary" | "outline" | "destructive"> =
    {
        Received: "outline",
        Diagnosing: "secondary",
        InProgress: "secondary",
        Ready: "default",
        Completed: "outline",
        Cancelled: "destructive",
    };

export function StatusBadge({ status, testId }: { status: Order["status"]; testId?: string }) {
    return (
        <Badge
            key={status}
            variant={statusVariant[status]}
            data-testid={testId}
            className="motion-safe:animate-in motion-safe:fade-in motion-safe:zoom-in-90 duration-200 ease-out"
        >
            {status}
        </Badge>
    );
}

export function OrderList({ orders, empty }: { orders: Order[]; empty: string }) {
    if (orders.length === 0) {
        return <p className="text-sm text-muted-foreground">{empty}</p>;
    }
    return (
        <ul className="divide-y overflow-hidden rounded-lg border">
            {orders.map((order) => (
                <li
                    key={order.id}
                    className={`flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-2 ${enter}`}
                >
                    <Link
                        to={`/orders/${order.id}`}
                        className="font-medium underline-offset-4 hover:underline"
                    >
                        {order.ticket_number}
                    </Link>
                    <StatusBadge status={order.status} />
                    <span className="ml-auto text-sm text-muted-foreground">
                        {money(order.settlement_cents)}
                    </span>
                </li>
            ))}
        </ul>
    );
}

export const PAGE_SIZE = 20;

export function Pager({
    page,
    offset,
    onOffset,
}: {
    page: Page<unknown> | undefined;
    offset: number;
    onOffset: (offset: number) => void;
}) {
    if (!page || page.total <= PAGE_SIZE) {
        return null;
    }
    const last = Math.min(offset + PAGE_SIZE, page.total);
    return (
        <div className="flex items-center justify-between gap-2 text-sm text-muted-foreground">
            <span>
                {offset + 1}–{last} of {page.total}
            </span>
            <div className="flex gap-2">
                <Button
                    variant="outline"
                    size="sm"
                    disabled={offset === 0}
                    onClick={() => onOffset(Math.max(0, offset - PAGE_SIZE))}
                >
                    Previous
                </Button>
                <Button
                    variant="outline"
                    size="sm"
                    disabled={last >= page.total}
                    onClick={() => onOffset(offset + PAGE_SIZE)}
                >
                    Next
                </Button>
            </div>
        </div>
    );
}

/** A label and value pair in a details list. */
export function Detail({
    label,
    children,
    testId,
}: {
    label: string;
    children: ReactNode;
    testId?: string;
}) {
    return (
        <div className="flex flex-col gap-0.5">
            <dt className="text-xs text-muted-foreground">{label}</dt>
            <dd className="text-sm" data-testid={testId}>
                {children}
            </dd>
        </div>
    );
}
