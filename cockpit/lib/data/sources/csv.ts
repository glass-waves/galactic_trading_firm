import { readFileSync, readdirSync } from "fs";
import { join } from "path";
import type { Trade } from "@/lib/types";

const DATA_DIR = join(process.cwd(), "..", "data");

interface CsvTrade {
  row_type: string;
  date: string;
  ticker: string;
  direction: string;
  entry_time: string;
  exit_time: string;
  entry_price: string;
  exit_price: string;
  size: string;
  pnl: string;
  pnl_pct: string;
  hold_duration_ms: string;
  exit_reason: string;
  entry_composite: string;
  exit_composite: string;
  entry_1m: string;
  entry_5m: string;
  entry_1h: string;
  exit_1m: string;
  exit_5m: string;
  exit_1h: string;
}

const CSV_HEADER =
  "row_type,date,ticker,direction,entry_time,exit_time,entry_price,exit_price,size,pnl,pnl_pct,hold_duration_ms,exit_reason,entry_composite,exit_composite,entry_1m,entry_5m,entry_1h,exit_1m,exit_5m,exit_1h,max_composite,max_composite_time,positive_ticks,total_ticks";

function parseCsvLine(header: string[], line: string): Record<string, string> {
  const values = line.split(",");
  const record: Record<string, string> = {};
  for (let i = 0; i < header.length; i++) {
    record[header[i]] = values[i] ?? "";
  }
  return record;
}

function csvTradeToTrade(row: Record<string, string>, idx: number): Trade {
  const parseNum = (v: string) => {
    const n = parseFloat(v);
    return isNaN(n) ? null : n;
  };

  return {
    id: idx,
    ticker: row.ticker,
    direction: row.direction?.toLowerCase() as "long" | "short",
    entry_price: parseFloat(row.entry_price) || 0,
    exit_price: parseFloat(row.exit_price) || 0,
    position_size: parseFloat(row.size) || 0,
    pnl_dollars: parseFloat(row.pnl) || 0,
    pnl_percent: parseNum(row.pnl_pct) as number,
    hold_duration_ms: parseInt(row.hold_duration_ms) || 0,
    exit_reason: row.exit_reason,
    entry_signal_at: row.entry_time,
    entry_fill_at: row.entry_time,
    exit_signal_at: row.exit_time,
    exit_fill_at: row.exit_time,
    entry_score_1min: parseNum(row.entry_1m),
    entry_score_5min: parseNum(row.entry_5m),
    entry_score_hourly: parseNum(row.entry_1h),
    entry_score_composite: parseNum(row.entry_composite),
    exit_score_1min: parseNum(row.exit_1m),
    exit_score_5min: parseNum(row.exit_5m),
    exit_score_hourly: parseNum(row.exit_1h),
    exit_score_composite: parseNum(row.exit_composite),
    high_water_mark: null,
    low_water_mark: null,
    config_version_id: null,
    is_paper: false,
  };
}

let cachedTrades: Trade[] | null = null;

export function loadAllCsvTrades(): Trade[] {
  if (cachedTrades) return cachedTrades;

  const files = readdirSync(DATA_DIR)
    .filter((f) => f.startsWith("backtest_") && f.endsWith("_trades.csv"))
    .sort();

  const allTrades: Trade[] = [];
  let globalIdx = 1;

  for (const file of files) {
    const content = readFileSync(join(DATA_DIR, file), "utf-8");
    const lines = content.split("\n");

    // find the header line
    const headerIdx = lines.findIndex((l) => l.startsWith("row_type,"));
    if (headerIdx === -1) continue;

    const header = lines[headerIdx].split(",");

    for (let i = headerIdx + 1; i < lines.length; i++) {
      const line = lines[i].trim();
      if (!line.startsWith("trade,")) continue;

      const row = parseCsvLine(header, line);
      allTrades.push(csvTradeToTrade(row, globalIdx++));
    }
  }

  // sort by exit time descending
  allTrades.sort(
    (a, b) =>
      new Date(b.exit_fill_at).getTime() - new Date(a.exit_fill_at).getTime()
  );

  cachedTrades = allTrades;
  return allTrades;
}

export function getCsvTradeStats(): {
  totalPnl: number;
  totalTrades: number;
  winRate: number;
  avgPnl: number;
} {
  const trades = loadAllCsvTrades();
  const totalTrades = trades.length;
  const totalPnl = trades.reduce((sum, t) => sum + t.pnl_dollars, 0);
  const winners = trades.filter((t) => t.pnl_dollars > 0).length;

  return {
    totalPnl,
    totalTrades,
    winRate: totalTrades > 0 ? (winners / totalTrades) * 100 : 0,
    avgPnl: totalTrades > 0 ? totalPnl / totalTrades : 0,
  };
}

export function getCsvTickerSummary(): {
  ticker: string;
  trades: number;
  win_rate: number;
  total_pnl: number;
  avg_pnl: number;
}[] {
  const trades = loadAllCsvTrades();
  const byTicker = new Map<
    string,
    { trades: number; winners: number; pnl: number }
  >();

  for (const t of trades) {
    const entry = byTicker.get(t.ticker) ?? { trades: 0, winners: 0, pnl: 0 };
    entry.trades++;
    if (t.pnl_dollars > 0) entry.winners++;
    entry.pnl += t.pnl_dollars;
    byTicker.set(t.ticker, entry);
  }

  return Array.from(byTicker.entries())
    .map(([ticker, v]) => ({
      ticker,
      trades: v.trades,
      win_rate:
        v.trades > 0
          ? Math.round((v.winners / v.trades) * 1000) / 10
          : 0,
      total_pnl: Math.round(v.pnl * 100) / 100,
      avg_pnl: v.trades > 0 ? Math.round((v.pnl / v.trades) * 100) / 100 : 0,
    }))
    .sort((a, b) => b.total_pnl - a.total_pnl);
}

export function getCsvDailyPnl(
  days?: number
): { day: string; pnl: number }[] {
  const trades = loadAllCsvTrades();
  const byDay = new Map<string, number>();

  for (const t of trades) {
    const day = t.exit_fill_at.slice(0, 10);
    byDay.set(day, (byDay.get(day) ?? 0) + t.pnl_dollars);
  }

  const result = Array.from(byDay.entries())
    .map(([day, pnl]) => ({ day, pnl: Math.round(pnl * 100) / 100 }))
    .sort((a, b) => b.day.localeCompare(a.day));

  return days ? result.slice(0, days) : result;
}

export function getCsvRecentTrades(limit = 10): Trade[] {
  return loadAllCsvTrades().slice(0, limit);
}
