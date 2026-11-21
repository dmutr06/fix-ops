import { type QueryClient, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { createContext, type ReactNode, useContext } from "react";
import { ApiError, api, call, type Role, type Staff } from "./api";

type Session = {
    staff: Staff;
    can: (...roles: Role[]) => boolean;
    signOut: () => void;
};

const SessionContext = createContext<Session | null>(null);

/** Replaces the signed-in staff member and drops every record cached for the previous one. */
export function setSession(queryClient: QueryClient, staff: Staff | null) {
    queryClient.removeQueries({ predicate: (query) => query.queryKey[0] !== "session" });
    queryClient.setQueryData(["session"], staff);
}

/** The signed-in staff member; only used below {@link SessionGate}. */
export function useSession(): Session {
    const session = useContext(SessionContext);
    if (!session) {
        throw new Error("useSession outside SessionGate");
    }
    return session;
}

export function useCurrentStaff() {
    return useQuery({
        queryKey: ["session"],
        queryFn: async () => {
            try {
                return await call(api.GET("/api/v1/sessions/current"));
            } catch (error) {
                if (error instanceof ApiError && error.status === 401) {
                    return null;
                }
                throw error;
            }
        },
        retry: false,
    });
}

export function useSignIn() {
    const queryClient = useQueryClient();
    return useMutation({
        mutationFn: (body: { username: string; password: string }) =>
            call(api.POST("/api/v1/sessions", { body })),
        onSuccess: (session) => setSession(queryClient, session.staff),
    });
}

/** Renders `signIn` until a session exists, then `children` with the session in context. */
export function SessionGate({ signIn, children }: { signIn: ReactNode; children: ReactNode }) {
    const queryClient = useQueryClient();
    const current = useCurrentStaff();
    const signOut = useMutation({
        mutationFn: () => call(api.DELETE("/api/v1/sessions/current")),
        onSettled: () => setSession(queryClient, null),
    });
    if (current.isPending) {
        return null;
    }
    const staff = current.data;
    if (!staff) {
        return signIn;
    }
    const session: Session = {
        staff,
        can: (...roles) => roles.includes(staff.role) || staff.role === "Administrator",
        signOut: () => signOut.mutate(),
    };
    return <SessionContext.Provider value={session}>{children}</SessionContext.Provider>;
}
