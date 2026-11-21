import { useQuery } from "@tanstack/react-query";
import { ErrorAlert, OrderList, Section } from "@/components/common";
import { ListSkeleton } from "@/components/motion";
import { api, callPage } from "@/lib/api";
import type { operations } from "@/lib/schema";
import { useSession } from "@/lib/session";

type Query = NonNullable<operations["listRepairOrders"]["parameters"]["query"]>;

function QueueSection({ title, query, empty }: { title: string; query: Query; empty: string }) {
    const orders = useQuery({
        queryKey: ["orders", query],
        queryFn: () =>
            callPage(api.GET("/api/v1/orders", { params: { query: { ...query, limit: 100 } } })),
    });
    return (
        <Section title={title}>
            {orders.isPending ? (
                <ListSkeleton />
            ) : (
                <OrderList orders={orders.data?.items ?? []} empty={empty} />
            )}
            <ErrorAlert error={orders.error} />
        </Section>
    );
}

/** The orders the signed-in role should act on next (REQ-F-023). */
export function QueuePage() {
    const { staff } = useSession();
    return (
        <>
            <h1 className="text-xl font-semibold">Work queue</h1>
            {staff.role === "Receptionist" && (
                <>
                    <QueueSection
                        title="Awaiting client decision"
                        query={{ awaiting_decision: true }}
                        empty="No estimate is waiting for the client."
                    />
                    <QueueSection
                        title="Ready for payment and pickup"
                        query={{ status: "Ready" }}
                        empty="No order is ready."
                    />
                </>
            )}
            {staff.role === "Technician" && (
                <QueueSection
                    title="My open orders"
                    query={{ assigned_technician_id: staff.id, open: true }}
                    empty="No open order is assigned to you."
                />
            )}
            {staff.role === "Administrator" && (
                <QueueSection title="Open orders" query={{ open: true }} empty="No open orders." />
            )}
        </>
    );
}
