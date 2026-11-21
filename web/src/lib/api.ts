import createClient from "openapi-fetch";
import type { components, paths } from "./schema";

export type Schemas = components["schemas"];
export type Order = Schemas["RepairOrder"];
export type Client = Schemas["Client"];
export type Device = Schemas["Device"];
export type Staff = Schemas["StaffAccount"];
export type AuditEntry = Schemas["AuditEntry"];
export type Role = Schemas["Role"];

/** The API on the same origin; the session cookie goes with every request. */
export const api = createClient<paths>({ baseUrl: "", credentials: "same-origin" });

/** A rejected request, with the service's message. */
export class ApiError extends Error {
    readonly status: number;

    constructor(status: number, message: string) {
        super(message);
        this.status = status;
    }
}

type Result<T> = { data?: T; error?: unknown; response: Response };

/** The data of a successful call; otherwise throws an {@link ApiError}. */
export async function call<T>(request: Promise<Result<T>>): Promise<T> {
    const { data, error, response } = await request;
    if (!response.ok) {
        const message = (error as { message?: string } | undefined)?.message ?? response.statusText;
        throw new ApiError(response.status, message);
    }
    return data as T;
}

/** A page of a list and the number of matches before paging (`X-Total-Count`). */
export type Page<T> = { items: T[]; total: number };

export async function callPage<T>(request: Promise<Result<T[]>>): Promise<Page<T>> {
    const result = await request;
    const items = await call(Promise.resolve(result));
    return { items, total: Number(result.response.headers.get("x-total-count") ?? items.length) };
}
