import { LogOutIcon, MoonIcon, SunIcon } from "lucide-react";
import type { ReactNode } from "react";
import { NavLink, useLocation } from "react-router";
import { FadeIn } from "@/components/motion";
import { useTheme } from "@/components/theme-provider";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useSession } from "@/lib/session";

const links = [
    { to: "/", label: "Queue", end: true },
    { to: "/orders", label: "Orders" },
    { to: "/clients", label: "Clients" },
];

export function Layout({ children }: { children: ReactNode }) {
    const { staff, signOut } = useSession();
    const { theme, setTheme } = useTheme();
    const { pathname } = useLocation();
    const nav =
        staff.role === "Administrator" ? [...links, { to: "/staff", label: "Staff" }] : links;
    return (
        <div className="min-h-svh bg-background">
            <header className="border-b">
                <div className="mx-auto flex max-w-5xl flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
                    <span className="font-semibold">FixOps</span>
                    <nav className="flex gap-1 overflow-x-auto">
                        {nav.map((link) => (
                            <NavLink
                                key={link.to}
                                to={link.to}
                                end={link.end}
                                className={({ isActive }) =>
                                    `rounded-md px-2.5 py-1.5 text-sm ${isActive ? "bg-muted font-medium" : "text-muted-foreground hover:text-foreground"}`
                                }
                            >
                                {link.label}
                            </NavLink>
                        ))}
                    </nav>
                    <div className="ml-auto flex items-center gap-2 text-sm">
                        <span className="hidden sm:inline">{staff.username}</span>
                        <Badge variant="secondary">{staff.role}</Badge>
                        <Button
                            variant="ghost"
                            size="icon"
                            aria-label="Toggle theme"
                            onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
                        >
                            {theme === "dark" ? <SunIcon /> : <MoonIcon />}
                        </Button>
                        <Button variant="outline" size="sm" onClick={signOut}>
                            <LogOutIcon />
                            Sign out
                        </Button>
                    </div>
                </div>
            </header>
            <FadeIn key={pathname} className="mx-auto flex max-w-5xl flex-col gap-6 px-4 py-6">
                <main className="contents">{children}</main>
            </FadeIn>
        </div>
    );
}
