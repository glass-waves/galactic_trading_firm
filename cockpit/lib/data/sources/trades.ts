import { query } from "@/lib/db";
import type { Trade, DailyPerformance } from "@/lib/types";
import type { TradeFilters } from "../types";

export async function getTrades(filters: TradeFilters = {}): Promise<Trade[]> {
  const conditions: string[] = [];
  const params: unknown[] = [];
  let paramIdx = 1;

  if (filters.ticker) {
    conditions.push(`ticker = $${paramIdx++}`);
    params.push(filters.ticker);
  }
  if (filters.exitReason) {
    conditions.push(`exit_reason = $${paramIdx++}`);
    params.push(filters.exitReason);
  }
  if (filters.configVersionId) {
    conditions.push(`config_version_id = $${paramIdx++}`);
    params.push(filters.configVersionId);
  }
  if (filters.dateRange?.start) {
    conditions.push(`exit_fill_at >= $${paramIdx++}`);
    params.push(filters.dateRange.start);
  }
  if (filters.dateRange?.end) {
    conditions.push(`exit_fill_at <= $${paramIdx++}`);
    params.push(filters.dateRange.end);
  }

  const where = conditions.length > 0 ? `WHERE ${conditions.join(" AND ")}` : "";
  const limit = filters.limit ?? 500;
  const offset = filters.offset ?? 0;

  return query<Trade>(
    `SELECT * FROM trades ${where} ORDER BY exit_fill_at DESC LIMIT $${paramIdx++} OFFSET $${paramIdx++}`,
    [...params, limit, offset]
  );
}

export async function getTradeStats(): Promise<{
  totalPnl: number;
  totalTrades: number;
  winRate: number;
  avgPnl: number;
}> {
  const rows = await query<{
    total_pnl: string;
    total_trades: string;
    winning_trades: string;
  }>(
    `SELECT
      COALESCE(SUM(pnl_dollars), 0) AS total_pnl,
      COUNT(*) AS total_trades,
      SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END) AS winning_trades
    FROM trades`
  );

  const row = rows[0];
  const totalTrades = Number(row.total_trades);
  const winningTrades = Number(row.winning_trades);

  return {
    totalPnl: Number(row.total_pnl),
    totalTrades,
    winRate: totalTrades > 0 ? (winningTrades / totalTrades) * 100 : 0,
    avgPnl: totalTrades > 0 ? Number(row.total_pnl) / totalTrades : 0,
  };
}

export async function getRecentTrades(limit = 10): Promise<Trade[]> {
  return query<Trade>(
    `SELECT * FROM trades ORDER BY exit_fill_at DESC LIMIT $1`,
    [limit]
  );
}

export async function getTickerSummary(): Promise<
  {
    ticker: string;
    trades: number;
    win_rate: number;
    total_pnl: number;
    avg_pnl: number;
  }[]
> {
  return query(
    `SELECT
      ticker,
      COUNT(*)::int AS trades,
      ROUND(SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::numeric / NULLIF(COUNT(*), 0) * 100, 1)::float AS win_rate,
      ROUND(SUM(pnl_dollars)::numeric, 2)::float AS total_pnl,
      ROUND(AVG(pnl_dollars)::numeric, 2)::float AS avg_pnl
    FROM trades
    GROUP BY ticker
    ORDER BY SUM(pnl_dollars) DESC`
  );
}

export async function getDailyPnl(days = 30): Promise<{ day: string; pnl: number }[]> {
  return query(
    `SELECT
      date_trunc('day', exit_fill_at)::date::text AS day,
      ROUND(SUM(pnl_dollars)::numeric, 2)::float AS pnl
    FROM trades
    GROUP BY day
    ORDER BY day DESC
    LIMIT $1`,
    [days]
  );
}
