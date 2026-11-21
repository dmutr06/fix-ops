import { MutationCache, QueryCache, QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./index.css";
import { App } from "@/App";
import { ThemeProvider } from "@/components/theme-provider";
import { ApiError } from "@/lib/api";
import { setSession } from "@/lib/session";

/** An expired or ended session sends the user back to sign-in. */
function onError(error: Error) {
    if (error instanceof ApiError && error.status === 401) {
        setSession(queryClient, null);
    }
}

const queryClient = new QueryClient({
    queryCache: new QueryCache({ onError }),
    mutationCache: new MutationCache({ onError }),
    defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});

const root = document.getElementById("root");
if (root) {
    createRoot(root).render(
        <StrictMode>
            <ThemeProvider>
                <QueryClientProvider client={queryClient}>
                    <App />
                </QueryClientProvider>
            </ThemeProvider>
        </StrictMode>,
    );
}
