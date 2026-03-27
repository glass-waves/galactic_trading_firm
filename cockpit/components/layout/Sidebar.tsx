"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const NAV_ITEMS = [
  { href: "/", label: "dashboard" },
  { href: "/trades", label: "trades" },
  { href: "/backtest", label: "backtest" },
  { href: "/config", label: "config" },
  { href: "/indicators", label: "indicators" },
  { href: "/pipeline", label: "pipeline" },
  { href: "/agents", label: "agents" },
  { href: "/windows", label: "windows" },
];

export function Sidebar() {
  const pathname = usePathname();

  return (
    <nav className="w-40 shrink-0 border-r border-border bg-bg h-screen sticky top-0 flex flex-col">
      <div className="px-3 py-3 border-b border-border">
        <span className="text-text-dim text-xs uppercase tracking-widest">
          cockpit
        </span>
      </div>
      <div className="flex flex-col py-1">
        {NAV_ITEMS.map((item) => {
          const active =
            item.href === "/"
              ? pathname === "/"
              : pathname.startsWith(item.href);
          return (
            <Link
              key={item.href}
              href={item.href}
              className={`px-3 py-1.5 text-sm transition-none ${
                active
                  ? "text-text border-l-2 border-cyan bg-surface"
                  : "text-text-dim hover:text-text border-l-2 border-transparent"
              }`}
            >
              {item.label}
            </Link>
          );
        })}
      </div>
      <div className="mt-auto px-3 py-2 border-t border-border">
        <span className="text-text-muted text-xs">v0.1.0</span>
      </div>
    </nav>
  );
}
