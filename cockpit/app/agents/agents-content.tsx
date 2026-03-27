"use client";

import { useState, useEffect } from "react";
import type { EvolutionCycle, AgentMemo, Belief } from "@/lib/types";
import { formatDate, formatDollars } from "@/lib/format";
import { colors, statusColor } from "@/lib/theme";

export function AgentsContent() {
  const [cycles, setCycles] = useState<EvolutionCycle[]>([]);
  const [memos, setMemos] = useState<AgentMemo[]>([]);
  const [beliefs, setBeliefs] = useState<Belief[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedMemo, setSelectedMemo] = useState<AgentMemo | null>(null);

  useEffect(() => {
    Promise.all([
      fetch("/api/agents/cycles").then((r) => (r.ok ? r.json() : [])),
      fetch("/api/agents/memos").then((r) => (r.ok ? r.json() : [])),
      fetch("/api/agents/beliefs").then((r) => (r.ok ? r.json() : [])),
    ])
      .then(([c, m, b]) => {
        setCycles(c);
        setMemos(m);
        setBeliefs(b);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  if (loading) return <div className="text-text-dim text-xs">loading...</div>;

  // group beliefs by category
  const beliefsByCategory = new Map<string, Belief[]>();
  for (const b of beliefs) {
    const cat = b.category ?? "uncategorized";
    if (!beliefsByCategory.has(cat)) beliefsByCategory.set(cat, []);
    beliefsByCategory.get(cat)!.push(b);
  }

  const hasData = cycles.length > 0 || memos.length > 0 || beliefs.length > 0;

  if (!hasData) {
    return (
      <div className="text-text-muted text-xs">
        no agent activity recorded yet. run the orchestrator to generate evolution cycles.
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* evolution cycles */}
      <div className="border border-border bg-surface p-3">
        <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
          evolution cycles ({cycles.length})
        </div>
        {cycles.length > 0 ? (
          <table className="w-full text-xs">
            <thead>
              <tr className="border-b border-border">
                <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                  date
                </th>
                <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                  type
                </th>
                <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                  model
                </th>
                <th className="text-right text-text-muted px-1 py-0.5 font-normal">
                  cost
                </th>
                <th className="text-right text-text-muted px-1 py-0.5 font-normal">
                  proposed
                </th>
                <th className="text-right text-text-muted px-1 py-0.5 font-normal">
                  promoted
                </th>
                <th className="text-right text-text-muted px-1 py-0.5 font-normal">
                  rejected
                </th>
              </tr>
            </thead>
            <tbody>
              {cycles.map((c) => (
                <tr key={c.id} className="border-b border-border hover:bg-bg">
                  <td className="px-1 py-0.5 text-text-dim">
                    {formatDate(c.trading_date)}
                  </td>
                  <td className="px-1 py-0.5 text-text">{c.cycle_type}</td>
                  <td className="px-1 py-0.5 text-text-dim">{c.model_used}</td>
                  <td className="px-1 py-0.5 text-right text-text-dim">
                    ${c.estimated_cost_usd?.toFixed(3) ?? "—"}
                  </td>
                  <td className="px-1 py-0.5 text-right text-text-dim">
                    {c.configs_proposed}
                  </td>
                  <td
                    className="px-1 py-0.5 text-right"
                    style={{
                      color: c.configs_promoted > 0 ? colors.green : colors.textDim,
                    }}
                  >
                    {c.configs_promoted}
                  </td>
                  <td
                    className="px-1 py-0.5 text-right"
                    style={{
                      color: c.configs_rejected > 0 ? colors.red : colors.textDim,
                    }}
                  >
                    {c.configs_rejected}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div className="text-text-muted text-xs">no cycles yet</div>
        )}
      </div>

      <div className="grid grid-cols-2 gap-px">
        {/* memos */}
        <div className="border border-border bg-surface p-3 max-h-96 overflow-y-auto">
          <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
            agent memos ({memos.length})
          </div>
          {memos.length > 0 ? (
            <div className="space-y-1">
              {memos.map((m) => (
                <div
                  key={m.id}
                  className={`px-2 py-1 text-xs cursor-pointer border-l-2 ${
                    selectedMemo?.id === m.id
                      ? "border-l-cyan bg-bg"
                      : "border-l-transparent hover:bg-bg"
                  }`}
                  onClick={() => setSelectedMemo(m)}
                >
                  <div className="flex items-baseline gap-2">
                    <span className="text-text-dim">
                      {formatDate(m.created_at)}
                    </span>
                    <span className="text-text">{m.agent}</span>
                    <span className="text-text-muted">{m.memo_type}</span>
                    {m.confidence_score != null && (
                      <span className="text-text-dim ml-auto">
                        conf:{m.confidence_score.toFixed(2)}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="text-text-muted text-xs">no memos yet</div>
          )}
        </div>

        {/* memo detail */}
        <div className="border border-border bg-surface p-3 max-h-96 overflow-y-auto">
          <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
            memo detail
          </div>
          {selectedMemo ? (
            <div className="text-xs space-y-2">
              <div className="grid grid-cols-4 gap-2">
                <div>
                  <span className="text-text-muted">confidence </span>
                  <span className="text-text">
                    {selectedMemo.confidence_score?.toFixed(2) ?? "—"}
                  </span>
                </div>
                <div>
                  <span className="text-text-muted">regime </span>
                  <span className="text-text">
                    {selectedMemo.volatility_regime ?? "—"}
                  </span>
                </div>
                <div>
                  <span className="text-text-muted">bias </span>
                  <span className="text-text">
                    {selectedMemo.directional_bias ?? "—"}
                  </span>
                </div>
                <div>
                  <span className="text-text-muted">signal </span>
                  <span className="text-text">
                    {selectedMemo.signal_quality ?? "—"}
                  </span>
                </div>
              </div>
              {selectedMemo.reasoning && (
                <div className="text-text-dim whitespace-pre-wrap border-t border-border pt-2 mt-2">
                  {selectedMemo.reasoning}
                </div>
              )}
              {selectedMemo.suggestions != null && (
                <div className="border-t border-border pt-2 mt-2">
                  <div className="text-text-muted mb-1">suggestions</div>
                  <pre className="text-text-dim overflow-auto">
                    {JSON.stringify(selectedMemo.suggestions as object, null, 2)}
                  </pre>
                </div>
              )}
            </div>
          ) : (
            <div className="text-text-muted text-xs">
              select a memo to view details
            </div>
          )}
        </div>
      </div>

      {/* beliefs */}
      <div className="border border-border bg-surface p-3">
        <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
          beliefs ({beliefs.length})
        </div>
        {beliefs.length > 0 ? (
          <div className="space-y-3">
            {[...beliefsByCategory.entries()].map(([category, bs]) => (
              <div key={category}>
                <div className="text-text-dim text-xs uppercase tracking-wide mb-1 border-b border-border pb-0.5">
                  {category}
                </div>
                {bs.map((b) => (
                  <div key={b.id} className="text-xs py-0.5 flex gap-3">
                    <span
                      className="w-20 shrink-0 uppercase"
                      style={{
                        color: statusColor(
                          b.status === "active"
                            ? "promoted"
                            : b.status === "disproven"
                              ? "rejected"
                              : "proposed"
                        ),
                      }}
                    >
                      {b.status}
                    </span>
                    <span className="text-text-dim w-10 shrink-0 text-right">
                      {b.confidence.toFixed(2)}
                    </span>
                    <span className="text-text-muted w-8 shrink-0 text-right">
                      e:{b.evidence_count}
                    </span>
                    <span className="text-text">{b.belief_text}</span>
                  </div>
                ))}
              </div>
            ))}
          </div>
        ) : (
          <div className="text-text-muted text-xs">no beliefs recorded yet</div>
        )}
      </div>
    </div>
  );
}
