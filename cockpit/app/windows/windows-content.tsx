"use client";

import { useState, useEffect } from "react";
import { colors } from "@/lib/theme";

interface Condition {
  type: string;
  timescale?: string;
  lead_by?: number;
  min_score?: number;
  max_score?: number;
  max_spread?: number;
  instance_id?: string;
}

interface WindowAction {
  action_type: string;
  instance_id: string;
  phase: string;
  enabled: boolean;
  priority: number;
  params: {
    name?: string;
    direction?: string;
    conditions?: Condition[];
  };
}

function formatCondition(c: Condition): string {
  switch (c.type) {
    case "timescale_min":
      return `${c.timescale} ≥ ${c.min_score}`;
    case "timescale_max":
      return `${c.timescale} ≤ ${c.max_score}`;
    case "timescale_lead":
      return `${c.timescale} leads by ${c.lead_by}`;
    case "timescale_spread_max":
      return `spread ≤ ${c.max_spread}`;
    case "timescale_all_min":
      return `all timescales ≥ ${c.min_score}`;
    case "indicator_min":
      return `${c.instance_id} ≥ ${c.min_score}`;
    case "indicator_max":
      return `${c.instance_id} ≤ ${c.max_score}`;
    default:
      return JSON.stringify(c);
  }
}

export function WindowsContent() {
  const [windows, setWindows] = useState<WindowAction[]>([]);
  const [rejectGates, setRejectGates] = useState<WindowAction[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasConfig, setHasConfig] = useState(false);

  useEffect(() => {
    fetch("/api/configs?view=latest-promoted")
      .then((res) => (res.ok ? res.json() : null))
      .then((config) => {
        if (config?.config_blob?.actions) {
          const actions = config.config_blob.actions as WindowAction[];
          setWindows(
            actions
              .filter((a) => a.action_type === "entry_window")
              .sort((a, b) => a.priority - b.priority)
          );
          setRejectGates(
            actions
              .filter((a) => a.action_type === "entry_reject_gate")
              .sort((a, b) => a.priority - b.priority)
          );
          setHasConfig(true);
        }
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  if (loading) return <div className="text-text-dim text-xs">loading...</div>;

  // show the hardcoded window definitions from backtest CLI if no windows in promoted config
  const showHardcoded = windows.length === 0 && rejectGates.length === 0;

  if (showHardcoded) {
    return (
      <div className="space-y-4">
        <div className="border border-border bg-surface p-3">
          <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
            entry windows (from backtest CLI — not yet in promoted config)
          </div>
          <div className="text-xs text-text-dim mb-3">
            run backtest with <span className="text-text">--use-entry-windows</span> to activate.
            promote a config with entry windows to see them here.
          </div>

          {/* hardcoded window definitions matching backtest/src/main.rs */}
          <div className="space-y-3">
            <WindowCard
              name="1m noise filter"
              type="reject gate"
              instanceId="reject_1m_noise"
              priority={0}
              enabled={true}
              conditions={[
                { type: "timescale_lead", timescale: "OneMinute", lead_by: 0.15 },
                { type: "timescale_max", timescale: "FiveMinute", max_score: 0.35 },
              ]}
              isReject
            />
            <WindowCard
              name="5m thrust"
              type="entry window"
              instanceId="window_5m_thrust"
              priority={10}
              enabled={true}
              conditions={[
                { type: "timescale_lead", timescale: "FiveMinute", lead_by: 0.15 },
                { type: "timescale_min", timescale: "FiveMinute", min_score: 0.50 },
                { type: "timescale_min", timescale: "OneHour", min_score: 0.0 },
              ]}
            />
            <WindowCard
              name="aligned bias"
              type="entry window"
              instanceId="window_aligned"
              priority={20}
              enabled={true}
              conditions={[
                { type: "timescale_spread_max", max_spread: 0.15 },
                { type: "timescale_all_min", min_score: 0.10 },
                { type: "indicator_min", instance_id: "mom_persist_5min", min_score: 0.0 },
              ]}
            />
            <WindowCard
              name="hourly trend"
              type="entry window"
              instanceId="window_1h_trend"
              priority={30}
              enabled={true}
              conditions={[
                { type: "timescale_lead", timescale: "OneHour", lead_by: 0.15 },
                { type: "timescale_min", timescale: "OneHour", min_score: 0.40 },
                { type: "timescale_min", timescale: "FiveMinute", min_score: 0.0 },
              ]}
            />
            <WindowCard
              name="strong core"
              type="entry window"
              instanceId="window_strong_core"
              priority={40}
              enabled={true}
              conditions={[
                { type: "timescale_min", timescale: "FiveMinute", min_score: 0.50 },
                { type: "timescale_min", timescale: "OneHour", min_score: 0.30 },
              ]}
            />
            <WindowCard
              name="engulfing reversal"
              type="entry window"
              instanceId="window_engulfing"
              priority={50}
              enabled={true}
              conditions={[
                { type: "indicator_min", instance_id: "candle_5min", min_score: 0.30 },
                { type: "timescale_min", timescale: "FiveMinute", min_score: 0.20 },
                { type: "timescale_min", timescale: "OneHour", min_score: 0.0 },
                { type: "indicator_max", instance_id: "adx_1hr", max_score: 0.25 },
              ]}
            />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* reject gates */}
      {rejectGates.length > 0 && (
        <div className="border border-border bg-surface p-3">
          <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
            reject gates ({rejectGates.length})
          </div>
          {rejectGates.map((g) => (
            <WindowCard
              key={g.instance_id}
              name={g.params.name ?? g.instance_id}
              type="reject gate"
              instanceId={g.instance_id}
              priority={g.priority}
              enabled={g.enabled}
              conditions={g.params.conditions ?? []}
              isReject
            />
          ))}
        </div>
      )}

      {/* entry windows */}
      <div className="border border-border bg-surface p-3">
        <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
          entry windows ({windows.length})
        </div>
        <div className="space-y-3">
          {windows.map((w) => (
            <WindowCard
              key={w.instance_id}
              name={w.params.name ?? w.instance_id}
              type="entry window"
              instanceId={w.instance_id}
              priority={w.priority}
              enabled={w.enabled}
              conditions={w.params.conditions ?? []}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function WindowCard({
  name,
  type,
  instanceId,
  priority,
  enabled,
  conditions,
  isReject,
}: {
  name: string;
  type: string;
  instanceId: string;
  priority: number;
  enabled: boolean;
  conditions: Condition[];
  isReject?: boolean;
}) {
  return (
    <div
      className="border bg-bg p-2"
      style={{
        borderColor: isReject ? colors.red : colors.borderAlt,
      }}
    >
      <div className="flex items-baseline gap-3 text-xs mb-1">
        <span className="text-text font-bold">{name}</span>
        <span className="text-text-muted">{instanceId}</span>
        <span className="text-text-muted">p:{priority}</span>
        <span
          style={{ color: enabled ? colors.green : colors.red }}
        >
          {enabled ? "enabled" : "disabled"}
        </span>
        <span
          className="ml-auto uppercase text-xs"
          style={{ color: isReject ? colors.red : colors.textMuted }}
        >
          {type}
        </span>
      </div>
      <div className="text-xs space-y-0.5">
        {conditions.map((c, i) => (
          <div key={i} className="flex gap-2">
            <span className="text-text-muted w-4">{isReject ? "✗" : "∧"}</span>
            <span className="text-text-dim">{formatCondition(c)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
