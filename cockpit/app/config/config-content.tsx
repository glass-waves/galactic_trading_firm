"use client";

import { useState, useEffect } from "react";
import type { ConfigVersion, ConfigChangelog } from "@/lib/types";
import { JsonTree } from "@/lib/panels/display/json-tree";
import { formatDate } from "@/lib/format";
import { statusColor, colors } from "@/lib/theme";

export function ConfigContent() {
  const [versions, setVersions] = useState<ConfigVersion[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [changelog, setChangelog] = useState<ConfigChangelog[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // load all config versions
  useEffect(() => {
    fetch("/api/configs")
      .then((res) => {
        if (!res.ok) throw new Error("fetch failed");
        return res.json();
      })
      .then((data) => {
        setVersions(data);
        if (data.length > 0) setSelectedId(data[0].id);
        setLoading(false);
      })
      .catch((err) => {
        setError(err.message);
        setLoading(false);
      });
  }, []);

  // load changelog
  useEffect(() => {
    fetch("/api/configs/changelog")
      .then((res) => (res.ok ? res.json() : []))
      .then(setChangelog)
      .catch(() => setChangelog([]));
  }, []);

  const selectedConfig = versions.find((v) => v.id === selectedId);

  if (loading) return <div className="text-text-dim text-xs">loading...</div>;
  if (error) return <div className="text-red text-xs">error: {error}</div>;

  return (
    <div className="flex gap-px h-[calc(100vh-80px)]">
      {/* left: version list */}
      <div className="w-80 shrink-0 border border-border bg-surface overflow-y-auto">
        <div className="text-text-muted text-xs uppercase tracking-wide px-2 py-1 border-b border-border">
          versions ({versions.length})
        </div>
        {versions.map((v) => (
          <div
            key={v.id}
            className={`px-2 py-1.5 cursor-pointer text-xs border-b border-border ${
              v.id === selectedId
                ? "border-l-2 bg-bg"
                : "border-l-2 border-l-transparent hover:bg-bg"
            }`}
            style={{
              borderLeftColor:
                v.id === selectedId ? colors.cyan : "transparent",
            }}
            onClick={() => setSelectedId(v.id)}
          >
            <div className="flex items-baseline gap-2">
              <span className="text-text">v{v.id}</span>
              <span
                className="text-xs uppercase"
                style={{ color: statusColor(v.status) }}
              >
                {v.status}
              </span>
              <span className="text-text-muted ml-auto">
                {formatDate(v.created_at)}
              </span>
            </div>
            {v.mutation_reason && (
              <div className="text-text-muted mt-0.5 truncate">
                {v.mutation_reason}
              </div>
            )}
            {v.backtest_sharpe != null && (
              <div className="text-text-muted mt-0.5">
                sharpe:{v.backtest_sharpe?.toFixed(2) ?? "—"} win:
                {v.backtest_win_rate != null
                  ? `${(v.backtest_win_rate * 100).toFixed(1)}%`
                  : "—"}{" "}
                trades:{v.backtest_total_trades ?? "—"}
              </div>
            )}
          </div>
        ))}
      </div>

      {/* right: detail panel */}
      <div className="flex-1 flex flex-col gap-px overflow-hidden">
        {/* config metadata */}
        {selectedConfig && (
          <div className="border border-border bg-surface p-3">
            <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
              config v{selectedConfig.id}
            </div>
            <div className="grid grid-cols-4 gap-2 text-xs">
              <div>
                <span className="text-text-muted">status </span>
                <span style={{ color: statusColor(selectedConfig.status) }}>
                  {selectedConfig.status}
                </span>
              </div>
              <div>
                <span className="text-text-muted">created </span>
                <span className="text-text-dim">
                  {formatDate(selectedConfig.created_at)}
                </span>
              </div>
              <div>
                <span className="text-text-muted">by </span>
                <span className="text-text-dim">
                  {selectedConfig.created_by ?? "—"}
                </span>
              </div>
              <div>
                <span className="text-text-muted">parent </span>
                <span className="text-text-dim">
                  {selectedConfig.parent_version_id
                    ? `v${selectedConfig.parent_version_id}`
                    : "—"}
                </span>
              </div>
            </div>
            {selectedConfig.mutation_reason && (
              <div className="text-xs text-text-dim mt-2">
                {selectedConfig.mutation_reason}
              </div>
            )}
          </div>
        )}

        {/* config blob */}
        <div className="border border-border bg-surface p-3 flex-1 overflow-y-auto">
          <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
            config blob
          </div>
          {selectedConfig?.config_blob ? (
            <JsonTree data={selectedConfig.config_blob} />
          ) : (
            <div className="text-text-muted text-xs">no config selected</div>
          )}
        </div>

        {/* changelog for this version */}
        {selectedConfig && changelog.length > 0 && (
          <div className="border border-border bg-surface p-3 max-h-48 overflow-y-auto">
            <div className="text-text-muted text-xs uppercase tracking-wide mb-2">
              changelog
            </div>
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                    category
                  </th>
                  <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                    tool
                  </th>
                  <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                    param
                  </th>
                  <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                    old → new
                  </th>
                  <th className="text-left text-text-muted px-1 py-0.5 font-normal">
                    reason
                  </th>
                </tr>
              </thead>
              <tbody>
                {changelog
                  .filter(
                    (c) => c.config_version_id === selectedConfig.id
                  )
                  .map((c) => (
                    <tr
                      key={c.id}
                      className="border-b border-border"
                    >
                      <td className="px-1 py-0.5 text-text-dim">
                        {c.change_category}
                      </td>
                      <td className="px-1 py-0.5 text-text-dim">
                        {c.target_tool_id ?? "—"}
                      </td>
                      <td className="px-1 py-0.5 text-text-dim">
                        {c.target_param ?? "—"}
                      </td>
                      <td className="px-1 py-0.5">
                        <span className="text-red">
                          {c.old_value != null
                            ? JSON.stringify(c.old_value)
                            : "—"}
                        </span>
                        <span className="text-text-muted"> → </span>
                        <span className="text-green">
                          {c.new_value != null
                            ? JSON.stringify(c.new_value)
                            : "—"}
                        </span>
                      </td>
                      <td className="px-1 py-0.5 text-text-muted truncate max-w-xs">
                        {c.reason ?? "—"}
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
