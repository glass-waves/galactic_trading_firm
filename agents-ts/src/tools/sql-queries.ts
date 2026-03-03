/**
 * parameterized queries for agent tools.
 *
 * all queries use parameterized sql — never arbitrary sql.
 * agents are read-only for trade data, write-only for memos and config proposals.
 */

import type pg from "pg";
import type {
  TradeRecord,
  DailyPerformance,
  ChangelogEntry,
  Belief,
} from "../models.js";

/** helper for building dynamic queries with positional params */
class QueryBuilder {
  private parts: string[] = [];
  private params: unknown[] = [];
  private idx = 0;

  constructor(base: string) {
    this.parts.push(base);
  }

  add(clause: string, value: unknown): this {
    this.idx++;
    this.parts.push(clause.replace("$?", `$${this.idx}`));
    this.params.push(value);
    return this;
  }

  /** append raw SQL with no params */
  raw(clause: string): this {
    this.parts.push(clause);
    return this;
  }

  /** add a LIMIT clause */
  limit(value: number): this {
    this.idx++;
    this.parts.push(`LIMIT $${this.idx}`);
    this.params.push(value);
    return this;
  }

  build(): { text: string; values: unknown[] } {
    return { text: this.parts.join(" "), values: this.params };
  }
}

export async function getRecentTrades(
  pool: pg.Pool,
  ticker?: string,
  limit = 20,
  since?: string,
): Promise<TradeRecord[]> {
  const qb = new QueryBuilder(`
    SELECT id, ticker, direction, entry_price, exit_price, position_size,
           pnl_dollars, pnl_percent, hold_duration_ms, exit_reason,
           entry_fill_at, exit_fill_at, entry_score_composite, config_version_id
    FROM trades
    WHERE 1=1
  `);

  if (ticker !== undefined) qb.add("AND ticker = $?", ticker);
  if (since !== undefined) qb.add("AND exit_fill_at >= $?", since);
  qb.raw("ORDER BY exit_fill_at DESC");
  qb.limit(limit);

  const { text, values } = qb.build();
  const result = await pool.query(text, values);
  return result.rows as TradeRecord[];
}

export async function getDailyPerformance(
  pool: pg.Pool,
  tradingDay?: string,
  ticker?: string,
): Promise<DailyPerformance[]> {
  const qb = new QueryBuilder(`
    SELECT
      date_trunc('day', entry_fill_at)::date AS trading_day,
      ticker,
      COUNT(*) AS total_trades,
      SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END) AS winning_trades,
      ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
      ROUND(SUM(pnl_dollars)::numeric, 2) AS total_pnl,
      ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
          / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
      ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms
    FROM trades
    WHERE 1=1
  `);

  if (tradingDay !== undefined)
    qb.add("AND date_trunc('day', entry_fill_at)::date = $?", tradingDay);
  if (ticker !== undefined) qb.add("AND ticker = $?", ticker);
  qb.raw("GROUP BY trading_day, ticker ORDER BY trading_day DESC");

  const { text, values } = qb.build();
  const result = await pool.query(text, values);
  return result.rows as DailyPerformance[];
}

export async function getConfigChangelog(
  pool: pg.Pool,
  since?: string,
  limit = 50,
): Promise<ChangelogEntry[]> {
  const qb = new QueryBuilder(`
    SELECT id, config_version_id, created_at, changed_by::text,
           change_category::text, target_timescale::text,
           target_tool_id, target_tool_type, target_param,
           old_value, new_value, reason
    FROM config_changelog
    WHERE 1=1
  `);

  if (since !== undefined) qb.add("AND created_at >= $?", since);
  qb.raw("ORDER BY created_at DESC");
  qb.limit(limit);

  const { text, values } = qb.build();
  const result = await pool.query(text, values);
  return result.rows as ChangelogEntry[];
}

export async function getPerformanceByExitReason(
  pool: pg.Pool,
  since?: string,
): Promise<Record<string, unknown>[]> {
  const qb = new QueryBuilder(`
    SELECT
      exit_reason::text,
      COUNT(*) AS total_trades,
      ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
      ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
          / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
      ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms
    FROM trades
    WHERE 1=1
  `);

  if (since !== undefined) qb.add("AND exit_fill_at >= $?", since);
  qb.raw("GROUP BY exit_reason");

  const { text, values } = qb.build();
  const result = await pool.query(text, values);
  return result.rows as Record<string, unknown>[];
}

export async function getCheckinMemosSinceLastPm(
  pool: pg.Pool,
  excludeCycleId?: number,
): Promise<Record<string, unknown>[]> {
  const qb = new QueryBuilder(`
    SELECT am.id, am.agent::text, am.evolution_cycle_id, am.memo_type::text,
           am.confidence_score, am.volatility_regime, am.directional_bias,
           am.signal_quality, am.flags, am.reasoning, am.created_at,
           am.trades_reviewed, am.period_win_rate, am.period_pnl,
           am.suggestions
    FROM agent_memos am
    WHERE am.memo_type IN ('observation', 'analysis')
      AND am.created_at > COALESCE(
          (SELECT MAX(ec.completed_at)
           FROM evolution_cycles ec
           WHERE ec.cycle_type = 'full_pm' AND ec.completed_at IS NOT NULL),
          '1970-01-01'::timestamptz
      )
  `);

  if (excludeCycleId !== undefined) {
    qb.add("AND am.evolution_cycle_id != $?", excludeCycleId);
  }

  qb.raw("ORDER BY am.created_at DESC");

  const { text, values } = qb.build();
  const result = await pool.query(text, values);
  return result.rows as Record<string, unknown>[];
}

export async function getAnalysisMemos(
  pool: pg.Pool,
  cycleId: number,
): Promise<Record<string, unknown>[]> {
  const result = await pool.query(
    `
    SELECT am.id, am.agent::text, am.evolution_cycle_id, am.memo_type::text,
           am.confidence_score, am.volatility_regime, am.directional_bias,
           am.signal_quality, am.flags, am.reasoning, am.created_at,
           am.trades_reviewed, am.period_win_rate, am.period_pnl,
           am.suggestions
    FROM agent_memos am
    WHERE am.memo_type = 'analysis'
      AND am.evolution_cycle_id = $1
    ORDER BY am.created_at ASC
    `,
    [cycleId],
  );
  return result.rows as Record<string, unknown>[];
}

export async function getActiveBeliefs(
  pool: pg.Pool,
  category?: string,
): Promise<Belief[]> {
  const qb = new QueryBuilder(`
    SELECT id, belief_text, confidence, evidence_count, category, status,
           source_memo_ids, created_at, updated_at
    FROM beliefs
    WHERE status = 'active'
  `);

  if (category !== undefined) qb.add("AND category = $?", category);
  qb.raw("ORDER BY confidence DESC, evidence_count DESC");

  const { text, values } = qb.build();
  const result = await pool.query(text, values);
  return result.rows as Belief[];
}

export async function writeBelief(
  pool: pg.Pool,
  belief: Belief,
): Promise<number> {
  const result = await pool.query(
    `
    INSERT INTO beliefs (
      belief_text, confidence, evidence_count, category, status, source_memo_ids
    ) VALUES (
      $1, $2, $3, $4, $5, $6
    )
    RETURNING id
    `,
    [
      belief.belief_text,
      belief.confidence,
      belief.evidence_count,
      belief.category,
      belief.status,
      belief.source_memo_ids,
    ],
  );
  return result.rows[0].id as number;
}
