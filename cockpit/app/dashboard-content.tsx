"use client";

import "@/lib/panels/register-all";

import { Panel } from "@/lib/panels/Panel";

export function DashboardContent() {
  return (
    <div className="space-y-px">
      {/* metrics strip — full width */}
      <Panel id="metrics-strip" />

      {/* main grid: 3 columns */}
      <div className="grid grid-cols-3 gap-px">
        {/* daily P&L bar chart — spans 2 cols */}
        <div className="col-span-2">
          <Panel id="daily-pnl" />
        </div>

        {/* ticker summary — 1 col */}
        <Panel id="ticker-summary" />
      </div>

      {/* calendar heatmap — full width */}
      <Panel id="calendar-heatmap" />

      {/* recent trades — full width */}
      <Panel id="recent-trades" />
    </div>
  );
}
