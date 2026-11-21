import { motion, type Transition } from "motion/react";
import type { ComponentProps } from "react";
import { Skeleton } from "@/components/ui/skeleton";

/** Short and soft, so steps never wait on an animation (REQ-NF-001). */
export const ease: Transition = { duration: 0.2, ease: [0.22, 1, 0.36, 1] };

/** Fades and lifts its content in when it mounts; remount with a new `key` to replay. */
export function FadeIn(props: ComponentProps<typeof motion.div>) {
    return (
        <motion.div
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            transition={ease}
            {...props}
        />
    );
}

/** Rows of a list: new rows fade in, removed rows fade out, the rest slide into place. */
export const listItem = {
    layout: "position" as const,
    initial: { opacity: 0, y: 4 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0 },
    transition: ease,
};

/** Placeholder rows with the height of a list row, so content does not jump in. */
export function ListSkeleton({ rows = 3 }: { rows?: number }) {
    return (
        <div className="flex flex-col gap-2" aria-hidden="true">
            {Array.from({ length: rows }, (_, row) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: placeholders have no identity
                <Skeleton key={row} className="h-10 w-full" />
            ))}
        </div>
    );
}

/** Placeholder for a details page: a title and a grid of fields. */
export function PageSkeleton() {
    return (
        <div className="flex flex-col gap-6" aria-hidden="true">
            <Skeleton className="h-7 w-64" />
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                {Array.from({ length: 8 }, (_, field) => (
                    // biome-ignore lint/suspicious/noArrayIndexKey: placeholders have no identity
                    <Skeleton key={field} className="h-10" />
                ))}
            </div>
            <ListSkeleton />
        </div>
    );
}
