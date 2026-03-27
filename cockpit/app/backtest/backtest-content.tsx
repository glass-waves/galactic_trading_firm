"use client";

import "@/lib/panels/register-all";

import { Panel } from "@/lib/panels/Panel";

export function BacktestContent() {
  return (
    <div className="space-y-px">
      {/* equity curve — full width */}
      <Panel id="equity-curve" />

      {/* trade scatter — full width */}
      <Panel id="trade-scatter" />

      {/* score distributions — 3 columns */}
      <div className="grid grid-cols-3 gap-px">
        <Panel id="score-distribution-composite" />
        <Panel id="score-distribution-5m" />
        <Panel id="score-distribution-1h" />
      </div>

      {/* exit reason breakdown — full width */}
      <Panel id="exit-breakdown" />
    </div>
  );
}
