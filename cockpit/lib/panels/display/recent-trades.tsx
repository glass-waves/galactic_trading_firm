"use client";

import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { formatDollars, formatTime, formatDuration } from "@/lib/format";
import { pnlColor } from "@/lib/theme";
import type { Trade } from "@/lib/types";

function RecentTradesComponent({ data }: PanelProps<Trade[]>) {
  if (data.length === 0) {
    return <div className="text-text-muted text-xs">no trades</div>;
  }

  return (
    <div className="space-y-0.5">
      {data.map((t) => (
        <div key={t.id} className="text-xs flex gap-2">
          <span className="text-text-muted">[{formatTime(t.exit_fill_at)}]</span>
          <span className="text-text w-12">{t.ticker}</span>
          <span className="text-text-dim w-10 uppercase">{t.direction}</span>
          <span style={{ color: pnlColor(t.pnl_dollars) }} className="w-16 text-right">
            {formatDollars(t.pnl_dollars)}
          </span>
          <span className="text-text-muted w-20">{t.exit_reason}</span>
          <span className="text-text-muted">{formatDuration(t.hold_duration_ms)}</span>
        </div>
      ))}
    </div>
  );
}

registerPanel<Trade[]>({
  id: "recent-trades",
  name: "recent trades",
  component: RecentTradesComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/trades?view=recent&limit=15");
    if (!res.ok) throw new Error("failed to fetch recent trades");
    return res.json();
  },
});
