/**
 * write structured memos to agent_memos table.
 *
 * provides tools for agents to persist structured observations
 * and recommendations for consumption by the PM agent.
 */

import type pg from "pg";
import type { AgentMemo } from "../models.js";

export async function writeMemo(
  pool: pg.Pool,
  memo: AgentMemo,
): Promise<number> {
  const result = await pool.query(
    `
    INSERT INTO agent_memos (
      agent, evolution_cycle_id, memo_type,
      confidence_score, volatility_regime, directional_bias,
      signal_quality, flags, reasoning,
      proposed_config_version_id,
      review_period_start, review_period_end,
      trades_reviewed, period_win_rate, period_sharpe, period_pnl,
      suggestions
    ) VALUES (
      $1::agent_type, $2, $3::memo_type,
      $4, $5, $6,
      $7, $8::jsonb, $9,
      $10,
      $11, $12,
      $13, $14, $15, $16,
      $17::jsonb
    )
    RETURNING id
    `,
    [
      memo.agent,
      memo.evolution_cycle_id,
      memo.memo_type,
      memo.confidence_score,
      memo.volatility_regime,
      memo.directional_bias,
      memo.signal_quality,
      JSON.stringify(memo.flags),
      memo.reasoning,
      memo.proposed_config_version_id,
      memo.review_period_start,
      memo.review_period_end,
      memo.trades_reviewed,
      memo.period_win_rate,
      memo.period_sharpe,
      memo.period_pnl,
      JSON.stringify(memo.suggestions),
    ],
  );
  return result.rows[0].id as number;
}
