CREATE VIEW daily_performance AS
SELECT
    date_trunc('day', entry_fill_at) AS trading_day,
    ticker,
    config_version_id,
    COUNT(*) AS total_trades,
    SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END) AS winning_trades,
    ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
    ROUND(SUM(pnl_dollars)::numeric, 2) AS total_pnl,
    ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
        / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
    ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms,
    ROUND(AVG(slippage_entry)::numeric, 6) AS avg_slippage_entry
FROM trades
WHERE NOT is_paper OR TRUE
GROUP BY date_trunc('day', entry_fill_at), ticker, config_version_id;

CREATE VIEW performance_by_exit_reason AS
SELECT
    exit_reason,
    COUNT(*) AS total_trades,
    ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
    ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
        / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
    ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms
FROM trades
GROUP BY exit_reason;

CREATE VIEW score_interaction_analysis AS
SELECT
    CASE
        WHEN entry_score_1min > 0.6 THEN 'strong'
        WHEN entry_score_1min > 0.3 THEN 'moderate'
        ELSE 'weak'
    END AS score_1min_bucket,
    CASE
        WHEN entry_score_5min > 0.6 THEN 'strong'
        WHEN entry_score_5min > 0.3 THEN 'moderate'
        ELSE 'weak'
    END AS score_5min_bucket,
    CASE
        WHEN entry_score_hourly > 0.6 THEN 'strong'
        WHEN entry_score_hourly > 0.3 THEN 'moderate'
        ELSE 'weak'
    END AS score_hourly_bucket,
    COUNT(*) AS total_trades,
    ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
    ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
        / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate
FROM trades
GROUP BY score_1min_bucket, score_5min_bucket, score_hourly_bucket
HAVING COUNT(*) >= 5;

CREATE VIEW recent_agent_signals AS
SELECT
    agent,
    memo_type,
    created_at,
    confidence_score,
    directional_bias,
    signal_quality,
    flags,
    trades_reviewed,
    period_win_rate,
    period_sharpe
FROM agent_memos
WHERE created_at > now() - interval '30 days'
ORDER BY agent, created_at DESC;

CREATE VIEW checkin_memos_since_last_pm AS
SELECT am.*
FROM agent_memos am
WHERE am.memo_type = 'observation'
  AND am.created_at > (
      SELECT MAX(ec.completed_at)
      FROM evolution_cycles ec
      WHERE ec.cycle_type = 'full_pm'
        AND ec.completed_at IS NOT NULL
  )
ORDER BY am.created_at ASC;

CREATE VIEW daily_cost_summary AS
SELECT
    trading_date,
    total_cost_usd,
    full_pm_cycles,
    checkin_cycles,
    budget_limit_usd,
    budget_exhausted,
    ROUND((total_cost_usd / NULLIF(budget_limit_usd, 0) * 100)::numeric, 1) AS budget_pct_used
FROM daily_budget
ORDER BY trading_date DESC;
