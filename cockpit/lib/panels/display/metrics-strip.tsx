"use client";

import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { formatDollars, formatPct, formatInt } from "@/lib/format";
import { pnlColor } from "@/lib/theme";

interface MetricItem {
  label: string;
  value: string;
  color?: string;
}

interface MetricsStripData {
  items: MetricItem[];
}

function MetricsStripComponent({ data }: PanelProps<MetricsStripData>) {
  return (
    <div className="flex items-baseline gap-4 flex-wrap">
      {data.items.map((item, i) => (
        <span key={i} className="flex items-baseline gap-1.5">
          <span className="text-text-muted text-xs">{item.label}</span>
          <span
            className="text-sm"
            style={{ color: item.color ?? "#E0E0E0" }}
          >
            {item.value}
          </span>
          {i < data.items.length - 1 && (
            <span className="text-text-muted ml-2">|</span>
          )}
        </span>
      ))}
    </div>
  );
}

registerPanel<MetricsStripData>({
  id: "metrics-strip",
  name: "metrics",
  component: MetricsStripComponent,
  defaultConfig: {},
  fetchData: async () => {
    const [statsRes, configRes, beliefsRes] = await Promise.all([
      fetch("/api/trades/stats"),
      fetch("/api/configs?view=latest-promoted"),
      fetch("/api/agents/beliefs?view=count"),
    ]);

    const stats = statsRes.ok
      ? await statsRes.json()
      : { totalPnl: 0, totalTrades: 0, winRate: 0, avgPnl: 0 };

    const config = configRes.ok ? await configRes.json() : null;
    const beliefsData = beliefsRes.ok ? await beliefsRes.json() : { count: 0 };

    return {
      items: [
        {
          label: "P&L",
          value: formatDollars(stats.totalPnl),
          color: pnlColor(stats.totalPnl),
        },
        {
          label: "trades",
          value: formatInt(stats.totalTrades),
        },
        {
          label: "win%",
          value: formatPct(stats.winRate),
        },
        {
          label: "avg",
          value: formatDollars(stats.avgPnl),
          color: pnlColor(stats.avgPnl),
        },
        {
          label: "config",
          value: config?.id ? `v${config.id}` : "—",
        },
        {
          label: "beliefs",
          value: formatInt(beliefsData.count ?? 0),
        },
      ],
    };
  },
});
