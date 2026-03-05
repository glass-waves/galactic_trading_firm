-- clean up agent_type and memo_type enums after consolidating from 7 agents to 2.
-- user approved truncating all dependent tables to allow enum recreation.

-- 1. drop views that depend on agent_type or memo_type columns
DROP VIEW IF EXISTS changelog_since;
DROP VIEW IF EXISTS changelog_by_timescale;
DROP VIEW IF EXISTS checkin_memos_since_last_pm;
DROP VIEW IF EXISTS recent_agent_signals;
DROP VIEW IF EXISTS daily_cost_summary;

-- 2. truncate all tables that reference agent_type or memo_type
TRUNCATE agent_memos, evolution_cycles, config_changelog, daily_budget CASCADE;
-- also clear trades for a clean slate
TRUNCATE trades CASCADE;

-- 3. recreate agent_type with only the values we need
-- postgres cannot drop enum values, so we must drop and recreate the type.
-- alter columns to text first, then drop the old type and create the new one.

ALTER TABLE agent_memos ALTER COLUMN agent TYPE text;
ALTER TABLE evolution_cycles ALTER COLUMN agents_triggered TYPE text[];
ALTER TABLE evolution_cycles ALTER COLUMN agents_completed TYPE text[];
ALTER TABLE config_changelog ALTER COLUMN changed_by TYPE text;

DROP TYPE agent_type;

CREATE TYPE agent_type AS ENUM (
    'agent_analysis',
    'agent_pm',
    'orchestrator'
);

ALTER TABLE agent_memos ALTER COLUMN agent TYPE agent_type USING agent::agent_type;
ALTER TABLE evolution_cycles ALTER COLUMN agents_triggered TYPE agent_type[] USING agents_triggered::agent_type[];
ALTER TABLE evolution_cycles ALTER COLUMN agents_completed TYPE agent_type[] USING agents_completed::agent_type[];
ALTER TABLE config_changelog ALTER COLUMN changed_by TYPE agent_type USING changed_by::agent_type;

-- 4. add 'analysis' to memo_type
ALTER TYPE memo_type ADD VALUE IF NOT EXISTS 'analysis';

-- 5. update agent_memos default (was 'recommendation', now 'observation' is safer)
ALTER TABLE agent_memos ALTER COLUMN memo_type SET DEFAULT 'observation';

-- 6. recreate all views we dropped
CREATE VIEW checkin_memos_since_last_pm AS
SELECT
    id, created_at, agent, evolution_cycle_id, memo_type,
    confidence_score, volatility_regime, directional_bias, signal_quality,
    flags, reasoning, proposed_config_version_id,
    review_period_start, review_period_end,
    trades_reviewed, period_win_rate, period_sharpe, period_pnl
FROM agent_memos am
WHERE memo_type = 'observation'::memo_type
  AND created_at > (SELECT max(ec.completed_at) FROM evolution_cycles ec
                    WHERE ec.cycle_type = 'full_pm'::cycle_type AND ec.completed_at IS NOT NULL)
ORDER BY created_at;

CREATE VIEW recent_agent_signals AS
SELECT agent, memo_type, created_at, confidence_score,
       directional_bias, signal_quality, flags,
       trades_reviewed, period_win_rate, period_sharpe
FROM agent_memos
WHERE created_at > (now() - '30 days'::interval)
ORDER BY agent, created_at DESC;

CREATE VIEW daily_cost_summary AS
SELECT trading_date, total_cost_usd, full_pm_cycles, checkin_cycles,
       budget_limit_usd, budget_exhausted,
       round((total_cost_usd / NULLIF(budget_limit_usd, 0::double precision) * 100::double precision)::numeric, 1) AS budget_pct_used
FROM daily_budget
ORDER BY trading_date DESC;

CREATE VIEW changelog_by_timescale AS
SELECT
    cl.*,
    cv.promoted_at,
    cv.status AS config_status
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status IN ('promoted', 'rolled_back')
ORDER BY cl.created_at DESC;

CREATE VIEW changelog_since AS
SELECT
    cl.*,
    cv.promoted_at
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status = 'promoted'
ORDER BY cv.promoted_at ASC, cl.id ASC;
