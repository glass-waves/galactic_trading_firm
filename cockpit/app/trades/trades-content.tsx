"use client";

import "@/lib/panels/register-all";

import { useState, useEffect, useMemo, useCallback, Fragment } from "react";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  getExpandedRowModel,
  flexRender,
  type ColumnDef,
  type SortingState,
  type ExpandedState,
  type Row,
} from "@tanstack/react-table";
import type { Trade } from "@/lib/types";
import { formatDollars, formatPct, formatTime, formatDate, formatDuration } from "@/lib/format";
import { pnlColor, colors } from "@/lib/theme";

function TradeDetail({ trade }: { trade: Trade }) {
  return (
    <div className="px-6 py-2 bg-bg border-b border-border text-xs space-y-1">
      <div className="grid grid-cols-4 gap-4">
        <div>
          <span className="text-text-muted">entry scores </span>
          <span className="text-text-dim">
            1m:{trade.entry_score_1min?.toFixed(2) ?? "—"}{" "}
            5m:{trade.entry_score_5min?.toFixed(2) ?? "—"}{" "}
            1h:{trade.entry_score_hourly?.toFixed(2) ?? "—"}{" "}
            comp:{trade.entry_score_composite?.toFixed(2) ?? "—"}
          </span>
        </div>
        <div>
          <span className="text-text-muted">exit scores </span>
          <span className="text-text-dim">
            1m:{trade.exit_score_1min?.toFixed(2) ?? "—"}{" "}
            5m:{trade.exit_score_5min?.toFixed(2) ?? "—"}{" "}
            1h:{trade.exit_score_hourly?.toFixed(2) ?? "—"}{" "}
            comp:{trade.exit_score_composite?.toFixed(2) ?? "—"}
          </span>
        </div>
        <div>
          <span className="text-text-muted">water marks </span>
          <span className="text-text-dim">
            high:{trade.high_water_mark?.toFixed(4) ?? "—"}{" "}
            low:{trade.low_water_mark?.toFixed(4) ?? "—"}
          </span>
        </div>
        <div>
          <span className="text-text-muted">config </span>
          <span className="text-text-dim">
            v{trade.config_version_id ?? "—"}
          </span>
          {trade.is_paper && (
            <span className="text-amber ml-2">[paper]</span>
          )}
        </div>
      </div>
      <div className="grid grid-cols-4 gap-4">
        <div>
          <span className="text-text-muted">entry </span>
          <span className="text-text-dim">
            signal:{trade.entry_signal_at ? formatTime(trade.entry_signal_at) : "—"}{" "}
            fill:{trade.entry_fill_at ? formatTime(trade.entry_fill_at) : "—"}
          </span>
        </div>
        <div>
          <span className="text-text-muted">exit </span>
          <span className="text-text-dim">
            signal:{trade.exit_signal_at ? formatTime(trade.exit_signal_at) : "—"}{" "}
            fill:{trade.exit_fill_at ? formatTime(trade.exit_fill_at) : "—"}
          </span>
        </div>
        <div>
          <span className="text-text-muted">size </span>
          <span className="text-text-dim">
            {trade.position_size?.toFixed(2) ?? "—"} shares
          </span>
        </div>
        <div>
          <span className="text-text-muted">P&L% </span>
          <span style={{ color: pnlColor(trade.pnl_percent ?? 0) }}>
            {trade.pnl_percent != null ? formatPct(trade.pnl_percent, 3) : "—"}
          </span>
        </div>
      </div>
    </div>
  );
}

const columns: ColumnDef<Trade, unknown>[] = [
  {
    id: "expander",
    header: "",
    size: 24,
    cell: ({ row }) => (
      <button
        onClick={(e) => {
          e.stopPropagation();
          row.toggleExpanded();
        }}
        className="text-text-muted hover:text-text"
      >
        {row.getIsExpanded() ? "▾" : "▸"}
      </button>
    ),
  },
  {
    accessorKey: "exit_fill_at",
    header: "date",
    size: 90,
    cell: ({ getValue }) => (
      <span className="text-text-dim">
        {formatDate(getValue() as string)}
      </span>
    ),
  },
  {
    accessorKey: "ticker",
    header: "ticker",
    size: 60,
  },
  {
    accessorKey: "direction",
    header: "dir",
    size: 50,
    cell: ({ getValue }) => (
      <span className="text-text-dim uppercase">{getValue() as string}</span>
    ),
  },
  {
    accessorKey: "entry_price",
    header: "entry",
    size: 80,
    cell: ({ getValue }) => (
      <span className="text-right tabular-nums">
        {(getValue() as number)?.toFixed(2)}
      </span>
    ),
  },
  {
    accessorKey: "exit_price",
    header: "exit",
    size: 80,
    cell: ({ getValue }) => (
      <span className="text-right tabular-nums">
        {(getValue() as number)?.toFixed(2)}
      </span>
    ),
  },
  {
    accessorKey: "pnl_dollars",
    header: "P&L",
    size: 80,
    cell: ({ getValue }) => {
      const v = getValue() as number;
      return (
        <span className="text-right tabular-nums" style={{ color: pnlColor(v) }}>
          {formatDollars(v)}
        </span>
      );
    },
  },
  {
    accessorKey: "hold_duration_ms",
    header: "hold",
    size: 60,
    cell: ({ getValue }) => (
      <span className="text-text-dim">
        {formatDuration(getValue() as number)}
      </span>
    ),
  },
  {
    accessorKey: "exit_reason",
    header: "exit reason",
    size: 120,
    cell: ({ getValue }) => (
      <span className="text-text-muted">{getValue() as string}</span>
    ),
  },
  {
    accessorKey: "entry_score_composite",
    header: "composite",
    size: 80,
    cell: ({ getValue }) => {
      const v = getValue() as number | null;
      return (
        <span className="text-text-dim tabular-nums">
          {v != null ? v.toFixed(3) : "—"}
        </span>
      );
    },
  },
];

export function TradesContent() {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sorting, setSorting] = useState<SortingState>([]);
  const [globalFilter, setGlobalFilter] = useState("");
  const [expanded, setExpanded] = useState<ExpandedState>({});

  // filters
  const [tickerFilter, setTickerFilter] = useState("");
  const [exitReasonFilter, setExitReasonFilter] = useState("");

  useEffect(() => {
    const params = new URLSearchParams({ limit: "2000" });
    if (tickerFilter) params.set("ticker", tickerFilter);
    if (exitReasonFilter) params.set("exit_reason", exitReasonFilter);

    setLoading(true);
    fetch(`/api/trades?${params}`)
      .then((res) => {
        if (!res.ok) throw new Error("fetch failed");
        return res.json();
      })
      .then((data) => {
        setTrades(data);
        setLoading(false);
      })
      .catch((err) => {
        setError(err.message);
        setLoading(false);
      });
  }, [tickerFilter, exitReasonFilter]);

  // extract unique tickers and exit reasons for filter dropdowns
  const tickers = useMemo(
    () => [...new Set(trades.map((t) => t.ticker))].sort(),
    [trades]
  );
  const exitReasons = useMemo(
    () => [...new Set(trades.map((t) => t.exit_reason))].sort(),
    [trades]
  );

  const table = useReactTable({
    data: trades,
    columns,
    state: { sorting, globalFilter, expanded },
    onSortingChange: setSorting,
    onGlobalFilterChange: setGlobalFilter,
    onExpandedChange: setExpanded,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    getExpandedRowModel: getExpandedRowModel(),
    initialState: { pagination: { pageSize: 50 } },
  });

  if (loading) {
    return <div className="text-text-dim text-xs">loading trades...</div>;
  }
  if (error) {
    return <div className="text-red text-xs">error: {error}</div>;
  }

  return (
    <div className="space-y-2">
      {/* filter bar */}
      <div className="flex items-center gap-3">
        <input
          type="text"
          value={globalFilter}
          onChange={(e) => setGlobalFilter(e.target.value)}
          placeholder="search..."
          className="bg-bg border border-border px-2 py-1 text-xs text-text w-40 focus:border-border-alt focus:outline-none"
        />
        <select
          value={tickerFilter}
          onChange={(e) => setTickerFilter(e.target.value)}
          className="bg-bg border border-border px-2 py-1 text-xs text-text focus:border-border-alt focus:outline-none"
        >
          <option value="">all tickers</option>
          {tickers.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <select
          value={exitReasonFilter}
          onChange={(e) => setExitReasonFilter(e.target.value)}
          className="bg-bg border border-border px-2 py-1 text-xs text-text focus:border-border-alt focus:outline-none"
        >
          <option value="">all exit reasons</option>
          {exitReasons.map((r) => (
            <option key={r} value={r}>
              {r}
            </option>
          ))}
        </select>
        <span className="ml-auto text-text-muted text-xs">
          {table.getFilteredRowModel().rows.length} trades
        </span>
      </div>

      {/* table */}
      <div className="overflow-x-auto">
        <table className="w-full text-xs">
          <thead>
            {table.getHeaderGroups().map((hg) => (
              <tr key={hg.id} className="border-b border-border">
                {hg.headers.map((header) => (
                  <th
                    key={header.id}
                    className="text-left text-text-muted px-2 py-1 cursor-pointer select-none uppercase tracking-wide font-normal"
                    style={{ width: header.getSize() }}
                    onClick={header.column.getToggleSortingHandler()}
                  >
                    {flexRender(
                      header.column.columnDef.header,
                      header.getContext()
                    )}
                    {{ asc: " ↑", desc: " ↓" }[
                      header.column.getIsSorted() as string
                    ] ?? ""}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <Fragment key={row.id}>
                <tr
                  className="border-b border-border hover:bg-surface cursor-pointer"
                  onClick={() => row.toggleExpanded()}
                >
                  {row.getVisibleCells().map((cell) => (
                    <td key={cell.id} className="px-2 py-1 whitespace-nowrap">
                      {flexRender(
                        cell.column.columnDef.cell,
                        cell.getContext()
                      )}
                    </td>
                  ))}
                </tr>
                {row.getIsExpanded() && (
                  <tr>
                    <td colSpan={columns.length}>
                      <TradeDetail trade={row.original} />
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
          </tbody>
        </table>
      </div>

      {/* pagination */}
      <div className="flex items-center gap-3 text-xs text-text-dim">
        <button
          onClick={() => table.previousPage()}
          disabled={!table.getCanPreviousPage()}
          className="disabled:text-text-muted hover:text-text"
        >
          ← prev
        </button>
        <span>
          {table.getState().pagination.pageIndex + 1} / {table.getPageCount()}
        </span>
        <button
          onClick={() => table.nextPage()}
          disabled={!table.getCanNextPage()}
          className="disabled:text-text-muted hover:text-text"
        >
          next →
        </button>
      </div>
    </div>
  );
}
