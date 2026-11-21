import { BrowserRouter, Navigate, Route, Routes } from "react-router";
import { Layout } from "@/components/layout";
import { Toaster } from "@/components/ui/sonner";
import { SessionGate, useSession } from "@/lib/session";
import { ClientPage } from "@/pages/client";
import { ClientsPage } from "@/pages/clients";
import { OrderPage } from "@/pages/order";
import { OrdersPage } from "@/pages/orders";
import { QueuePage } from "@/pages/queue";
import { SignInPage } from "@/pages/sign-in";
import { StaffPage } from "@/pages/staff";

function Pages() {
    const { staff } = useSession();
    return (
        <Layout>
            <Routes>
                <Route path="/" element={<QueuePage />} />
                <Route path="/orders" element={<OrdersPage />} />
                <Route path="/orders/:orderId" element={<OrderPage />} />
                <Route path="/clients" element={<ClientsPage />} />
                <Route path="/clients/:clientId" element={<ClientPage />} />
                {staff.role === "Administrator" && <Route path="/staff" element={<StaffPage />} />}
                <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
        </Layout>
    );
}

/** The staff web app (REQ-F-021): sign-in, then the pages of the signed-in role. */
export function App() {
    return (
        <BrowserRouter>
            <SessionGate signIn={<SignInPage />}>
                <Pages />
            </SessionGate>
            <Toaster />
        </BrowserRouter>
    );
}
