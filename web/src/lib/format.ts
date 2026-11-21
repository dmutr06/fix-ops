/** `12550` → `$125.50`. Amounts are whole cents (SRS §3.2). */
export function money(cents: number): string {
    const sign = cents < 0 ? "-" : "";
    const abs = Math.abs(cents);
    return `${sign}$${Math.floor(abs / 100)}.${String(abs % 100).padStart(2, "0")}`;
}

/** `125.50` → `12550`; `null` when the text is not a non-negative amount with at most two decimals. */
export function cents(dollars: string): number | null {
    const match = /^(\d+)(?:\.(\d{1,2}))?$/.exec(dollars.trim());
    if (!match) {
        return null;
    }
    return Number(match[1]) * 100 + Number((match[2] ?? "").padEnd(2, "0"));
}

/** Cents as an editable dollar amount: `15000` → `150.00`. */
export function dollars(cents: number): string {
    return money(cents).slice(1);
}

export function dateTime(value: string): string {
    return new Date(value).toLocaleString("en-US", { dateStyle: "medium", timeStyle: "short" });
}
