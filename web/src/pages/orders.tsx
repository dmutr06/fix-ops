import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { ErrorAlert, OrderList, PAGE_SIZE, Pager } from "@/components/common";
import { ListSkeleton } from "@/components/motion";
import { Input } from "@/components/ui/input";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { api, callPage, type Order } from "@/lib/api";

const STATUSES: Order["status"][] = [
    "Received",
    "Diagnosing",
    "InProgress",
    "Ready",
    "Completed",
    "Cancelled",
];

export function OrdersPage() {
    const [status, setStatus] = useState<Order["status"] | "">("");
    const [ticket, setTicket] = useState("");
    const [offset, setOffset] = useState(0);
    const query = {
        status: status || undefined,
        ticket_number: ticket.trim() || undefined,
        limit: PAGE_SIZE,
        offset,
    };
    const orders = useQuery({
        queryKey: ["orders", query],
        placeholderData: keepPreviousData,
        queryFn: () => callPage(api.GET("/api/v1/orders", { params: { query } })),
    });
    return (
        <>
            <h1 className="text-xl font-semibold">Orders</h1>
            <div className="flex flex-wrap gap-2">
                <NativeSelect
                    aria-label="Status"
                    value={status}
                    onChange={(event) => {
                        setStatus(event.target.value as Order["status"] | "");
                        setOffset(0);
                    }}
                >
                    <NativeSelectOption value="">All statuses</NativeSelectOption>
                    {STATUSES.map((value) => (
                        <NativeSelectOption key={value} value={value}>
                            {value}
                        </NativeSelectOption>
                    ))}
                </NativeSelect>
                <Input
                    aria-label="Ticket number"
                    placeholder="Ticket number"
                    className="w-48"
                    value={ticket}
                    onChange={(event) => {
                        setTicket(event.target.value);
                        setOffset(0);
                    }}
                />
            </div>
            {orders.isPending ? (
                <ListSkeleton />
            ) : (
                <OrderList orders={orders.data?.items ?? []} empty="No orders match." />
            )}
            <Pager page={orders.data} offset={offset} onOffset={setOffset} />
            <ErrorAlert error={orders.error} />
        </>
    );
}
