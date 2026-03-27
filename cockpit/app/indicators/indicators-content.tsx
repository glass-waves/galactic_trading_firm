"use client";

import { useState, useEffect, useMemo } from "react";
import { colors } from "@/lib/theme";

interface IndicatorConfig {
  indicator_type: string;
  instance_id: string;
  timescale: string;
  enabled: boolean;
  weight: number;
  params: Record<string, unknown>;
}

interface ActionConfig {
  action_type: string;
  instance_id: string;
  phase: string;
  enabled: boolean;
  priority: number;
  params: Record<string, unknown>;
}

interface ScoringConfig {
  timescale_weights: Record<string, number>;
  entry_threshold: number;
  exit_threshold: number;
  aggregation: string;
  hard_gate_timescales: string[];
  [key: string]: unknown;
}

export function IndicatorsContent() {
  const [indicators, setIndicators] = useState<IndicatorConfig[]>([]);
  const [actions, setActions] = useState<ActionConfig[]>([]);
  const [scoring, setScoring] = useState<ScoringConfig | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch("/api/configs?view=latest-promoted")
      .then((res) => (res.ok ? res.json() : null))
      .then((config) => {
        if (config?.config_blob) {
          setIndicators(config.config_blob.indicators ?? []);
          setActions(config.config_blob.actions ?? []);
          setScoring(config.config_blob.scoring ?? null);
        }
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  // group indicators by timescale
  const byTimescale = useMemo(() => {
    const groups = new Map<string, IndicatorConfig[]>();
    for (const ind of indicators) {
      const ts = ind.timescale;
      if (!groups.has(ts)) groups.set(ts, []);
      groups.get(ts)!.push(ind);
    }
    // sort each group by weight descending
    for (const [, inds] of groups) {
      inds.sort((a, b) => b.weight - a.weight);
    }
    return groups;
  }, [indicators]);

  // group actions by phase
  const byPhase = useMemo(() => {
    const groups = new Map<string, ActionConfig[]>();
    for (const act of actions) {
      if (!groups.has(act.phase)) groups.set(act.phase, []);
      groups.get(act.phase)!.push(act);
    }
    for (const [, acts] of groups) {
      acts.sort((a, b) => a.priority - b.priority);
    }
    return groups;
  }, [actions]);

  const maxWeight = useMemo(
    () => Math.max(...indicators.map((i) => i.weight), 0.01),
    [indicators]
  );

  if (loading) return <div className="text-text-dim text-xs">loading...</div>;

  const timescaleOrder = ["OneMinute", "FiveMinute", "OneHour", "OneDay", "OneMonth"];
  const sortedTimescales = [...byTimescale.keys()].sort(
    (a, b) => timescaleOrder.indexOf(a) - timescaleOrder.indexOf(b)
  );

  return (
    <div className="space-y-4">
      {/* scoring overview */}
      {scoring && (
        <div className="border border-border bg-surface p-3">
          <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
            scoring config
          </div>
          <div className="flex gap-6 text-xs">
            <div>
              <span className="text-text-muted">aggregation </span>
              <span className="text-text">{scoring.aggregation}</span>
            </div>
            <div>
              <span className="text-text-muted">entry_threshold </span>
              <span className="text-text">{scoring.entry_threshold}</span>
            </div>
            <div>
              <span className="text-text-muted">exit_threshold </span>
              <span className="text-text">{scoring.exit_threshold}</span>
            </div>
            <div>
              <span className="text-text-muted">hard_gates </span>
              <span className="text-text">
                {scoring.hard_gate_timescales?.join(", ") ?? "none"}
              </span>
            </div>
          </div>
          <div className="flex gap-4 text-xs mt-1">
            {Object.entries(scoring.timescale_weights ?? {}).map(
              ([ts, w]) => (
                <div key={ts}>
                  <span className="text-text-muted">{ts} </span>
                  <span className="text-text">{(w as number).toFixed(2)}</span>
                </div>
              )
            )}
          </div>
        </div>
      )}

      {/* indicators by timescale */}
      <div className="grid grid-cols-1 gap-px">
        {sortedTimescales.map((ts) => {
          const inds = byTimescale.get(ts) ?? [];
          const tsWeight = scoring?.timescale_weights?.[ts];
          return (
            <div key={ts} className="border border-border bg-surface p-3">
              <div className="text-text-muted text-xs uppercase tracking-wide mb-2 flex items-baseline gap-2">
                <span>{ts}</span>
                {tsWeight != null && (
                  <span className="text-text-dim">
                    (w={typeof tsWeight === "number" ? tsWeight.toFixed(2) : tsWeight})
                  </span>
                )}
                <span className="text-text-dim">{inds.length} indicators</span>
              </div>
              <table className="w-full text-xs">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left text-text-muted px-1 py-0.5 font-normal w-40">
                      instance_id
                    </th>
                    <th className="text-left text-text-muted px-1 py-0.5 font-normal w-32">
                      type
                    </th>
                    <th className="text-left text-text-muted px-1 py-0.5 font-normal w-16">
                      weight
                    </th>
                    <th className="text-left text-text-muted px-1 py-0.5 font-normal w-48">
                      bar
                    </th>
                    <th className="text-left text-text-muted px-1 py-0.5 font-normal w-12">
                      on
                    </th>
                    <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                      params
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {inds.map((ind) => (
                    <tr
                      key={ind.instance_id}
                      className="border-b border-border"
                    >
                      <td className="px-1 py-0.5 text-text">
                        {ind.instance_id}
                      </td>
                      <td className="px-1 py-0.5 text-text-dim">
                        {ind.indicator_type}
                      </td>
                      <td className="px-1 py-0.5 text-text tabular-nums">
                        {ind.weight.toFixed(2)}
                      </td>
                      <td className="px-1 py-0.5">
                        <div
                          className="h-2"
                          style={{
                            width: `${(ind.weight / maxWeight) * 100}%`,
                            backgroundColor: ind.enabled
                              ? colors.text
                              : colors.textMuted,
                            opacity: ind.enabled ? 0.6 : 0.2,
                          }}
                        />
                      </td>
                      <td className="px-1 py-0.5">
                        <span
                          style={{
                            color: ind.enabled ? colors.green : colors.red,
                          }}
                        >
                          {ind.enabled ? "✓" : "✗"}
                        </span>
                      </td>
                      <td className="px-1 py-0.5 text-text-muted truncate max-w-xs">
                        {Object.entries(ind.params)
                          .map(([k, v]) => `${k}:${JSON.stringify(v)}`)
                          .join(" ")}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          );
        })}
      </div>

      {/* actions by phase */}
      <div className="border border-border bg-surface p-3">
        <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
          actions
        </div>
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-border">
              <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                phase
              </th>
              <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                instance_id
              </th>
              <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                type
              </th>
              <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                priority
              </th>
              <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                on
              </th>
              <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                params
              </th>
            </tr>
          </thead>
          <tbody>
            {["Entry", "Monitor", "Exit", "Sizing"].flatMap((phase) =>
              (byPhase.get(phase) ?? []).map((act) => (
                <tr
                  key={act.instance_id}
                  className="border-b border-border"
                >
                  <td className="px-1 py-0.5 text-text-dim">{act.phase}</td>
                  <td className="px-1 py-0.5 text-text">
                    {act.instance_id}
                  </td>
                  <td className="px-1 py-0.5 text-text-dim">
                    {act.action_type}
                  </td>
                  <td className="px-1 py-0.5 text-text-dim tabular-nums">
                    {act.priority}
                  </td>
                  <td className="px-1 py-0.5">
                    <span
                      style={{
                        color: act.enabled ? colors.green : colors.red,
                      }}
                    >
                      {act.enabled ? "✓" : "✗"}
                    </span>
                  </td>
                  <td className="px-1 py-0.5 text-text-muted truncate max-w-md">
                    {Object.entries(act.params)
                      .map(([k, v]) => `${k}:${JSON.stringify(v)}`)
                      .join(" ")}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
