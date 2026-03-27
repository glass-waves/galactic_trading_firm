"use client";

import { registerPanel } from "../registry";
import type { PanelProps } from "../types";
import { formatDollars, formatPct, formatInt } from "@/lib/format";
import { pnlColor } from "@/lib/theme";

interface TickerRow {
  ticker: string;
  trades: number;
  win_rate: number;
  total_pnl: number;
  avg_pnl: number;
}

function TickerSummaryComponent({ data }: PanelProps<TickerRow[]>) {
  return (
    <table className="w-full text-xs">
      <thead>
        <tr className="border-b border-border">
          <th className="text-left text-text-muted px-2 py-1 font-normal uppercase tracking-wide">
            ticker
          </th>
          <th className="text-right text-text-muted px-2 py-1 font-normal uppercase tracking-wide">
            trades
          </th>
          <th className="text-right text-text-muted px-2 py-1 font-normal uppercase tracking-wide">
            win%
          </th>
          <th className="text-right text-text-muted px-2 py-1 font-normal uppercase tracking-wide">
            P&L
          </th>
          <th className="text-right text-text-muted px-2 py-1 font-normal uppercase tracking-wide">
            avg
          </th>
        </tr>
      </thead>
      <tbody>
        {data.map((row) => (
          <tr key={row.ticker} className="border-b border-border hover:bg-surface">
            <td className="px-2 py-1 text-text">{row.ticker}</td>
            <td className="px-2 py-1 text-right text-text-dim">
              {formatInt(row.trades)}
            </td>
            <td className="px-2 py-1 text-right text-text-dim">
              {formatPct(row.win_rate)}
            </td>
            <td
              className="px-2 py-1 text-right"
              style={{ color: pnlColor(row.total_pnl) }}
            >
              {formatDollars(row.total_pnl)}
            </td>
            <td
              className="px-2 py-1 text-right"
              style={{ color: pnlColor(row.avg_pnl) }}
            >
              {formatDollars(row.avg_pnl)}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

registerPanel<TickerRow[]>({
  id: "ticker-summary",
  name: "per ticker",
  component: TickerSummaryComponent,
  defaultConfig: {},
  fetchData: async () => {
    const res = await fetch("/api/trades?view=ticker-summary");
    if (!res.ok) throw new Error("failed to fetch ticker summary");
    return res.json();
  },
});
