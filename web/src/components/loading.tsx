import { Skeleton } from "@/components/ui/skeleton";

/**
 * CSS enter animation (tw-animate-css): a short fade and lift when an element mounts. Remount
 * with a new `key` to replay. Skipped when the system asks for reduced motion.
 */
export const enter =
    "motion-safe:animate-in motion-safe:fade-in motion-safe:slide-in-from-bottom-1 duration-200 ease-out";

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
