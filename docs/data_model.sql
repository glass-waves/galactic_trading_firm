-- ============================================================================
-- ADAPTIVE MULTI-TIMESCALE TRADING SYSTEM — DATA MODEL
-- ============================================================================
-- Version: 0.3
-- Database: Postgres (SQLite-compatible with minor type adjustments)
--
-- Design principles:
--   - Config is immutable/append-only (never update in place)
--   - Trade events capture full decision context at time of trade
--   - Indicator values at trade time are cached; all other price data
--     is reconstructed from historical feed on demand
--   - Agent memos are persisted for longitudinal analysis
--   - Two-tier evolution: full PM cycles + lightweight check-ins
--   - All timestamps are UTC microsecond precision
-- ============================================================================


-- ============================================================================
-- ENUMS
-- ============================================================================

CREATE TYPE timescale AS ENUM ('1min', '5min', '1hour', '1day', '1month');

CREATE TYPE trade_direction AS ENUM ('long', 'short');

CREATE TYPE exit_reason AS ENUM (
    'filter_alignment',   -- exit filters aligned normally
    'trailing_stop',      -- trailing stop triggered
    'hard_stop',          -- hard stop loss hit
    'max_hold_timeout',   -- exceeded maximum hold duration
    'take_profit',        -- take profit target reached
    'session_close',      -- market close forced exit
    'manual_override',    -- human intervention
    'config_change'       -- config reload forced position close
);

CREATE TYPE agent_type AS ENUM (
    'agent_1min',
    'agent_5min',
    'agent_hourly',
    'agent_daily',
    'agent_monthly',
    'agent_pm',
    'orchestrator'
);

CREATE TYPE memo_type AS ENUM (
    'observation',         -- lightweight check-in: read-only, no config authority
    'recommendation'       -- full PM cycle: may propose config changes
);

CREATE TYPE cycle_type AS ENUM (
    'full_pm',             -- full PM-orchestrated evolution cycle (1-3x daily)
    'checkin'              -- lightweight observation-only cycle (more frequent)
);

CREATE TYPE mutation_status AS ENUM (
    'proposed',           -- agent proposed, not yet backtested
    'backtesting',        -- currently being validated
    'validated',          -- passed backtest, awaiting promotion
    'promoted',           -- live in execution engine
    'rejected',           -- failed backtest validation
    'rolled_back',        -- was promoted but performance degraded
    'superseded'          -- replaced by a newer version
);


-- ============================================================================
-- STRATEGY CONFIGURATION (immutable append-only)
-- ============================================================================
-- Every config change produces a new row. The execution engine reads
-- the latest 'promoted' version. Rollback = promote an older version.
-- ============================================================================

CREATE TABLE config_versions (
    id                  BIGSERIAL PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    status              mutation_status NOT NULL DEFAULT 'proposed',
    promoted_at         TIMESTAMPTZ,
    rolled_back_at      TIMESTAMPTZ,

    -- Who created this version and why
    created_by          agent_type NOT NULL,
    parent_version_id   BIGINT REFERENCES config_versions(id),
    mutation_reason     TEXT NOT NULL,          -- freeform: why the agent made this change

    -- The full config snapshot as JSON. Schema will evolve.
    -- Contains all filter params, scoring weights, tooling params.
    config_blob         JSONB NOT NULL,

    -- Backtest results for this config (populated after validation)
    backtest_sharpe     DOUBLE PRECISION,
    backtest_win_rate   DOUBLE PRECISION,
    backtest_total_trades INTEGER,
    backtest_period_start TIMESTAMPTZ,
    backtest_period_end   TIMESTAMPTZ,
    backtest_results    JSONB                   -- detailed backtest output
);

CREATE INDEX idx_config_status ON config_versions(status);
CREATE INDEX idx_config_created_by ON config_versions(created_by);
CREATE INDEX idx_config_created_at ON config_versions(created_at DESC);

-- View: the currently active config
CREATE VIEW active_config AS
SELECT * FROM config_versions
WHERE status = 'promoted'
ORDER BY promoted_at DESC
LIMIT 1;


-- ============================================================================
-- TRADE EVENTS
-- ============================================================================
-- One row per completed trade (entry + exit).
-- Captures the full decision context at both entry and exit.
-- ============================================================================

CREATE TABLE trades (
    id                  BIGSERIAL PRIMARY KEY,
    ticker              VARCHAR(10) NOT NULL,
    direction           trade_direction NOT NULL,

    -- Timestamps (microsecond precision)
    entry_signal_at     TIMESTAMPTZ NOT NULL,   -- when filters aligned for entry
    entry_fill_at       TIMESTAMPTZ NOT NULL,   -- when broker confirmed fill
    exit_signal_at      TIMESTAMPTZ NOT NULL,   -- when exit condition triggered
    exit_fill_at        TIMESTAMPTZ NOT NULL,   -- when broker confirmed exit fill

    -- Prices
    entry_price         DOUBLE PRECISION NOT NULL,
    exit_price          DOUBLE PRECISION NOT NULL,
    position_size       DOUBLE PRECISION NOT NULL,  -- number of shares/contracts

    -- Outcome
    pnl_dollars         DOUBLE PRECISION NOT NULL,
    pnl_percent         DOUBLE PRECISION NOT NULL,
    commission          DOUBLE PRECISION NOT NULL DEFAULT 0,
    slippage_entry      DOUBLE PRECISION,       -- signal price vs fill price
    slippage_exit       DOUBLE PRECISION,
    hold_duration_ms    BIGINT NOT NULL,         -- exit_fill_at - entry_fill_at

    -- Why did we exit?
    exit_reason         exit_reason NOT NULL,

    -- Config context
    config_version_id   BIGINT NOT NULL REFERENCES config_versions(id),

    -- Per-timescale scores at ENTRY (what the decision tree saw)
    entry_score_1min    DOUBLE PRECISION,
    entry_score_5min    DOUBLE PRECISION,
    entry_score_hourly  DOUBLE PRECISION,
    entry_score_daily   DOUBLE PRECISION,
    entry_score_monthly DOUBLE PRECISION,
    entry_score_composite DOUBLE PRECISION NOT NULL,  -- final aggregated score

    -- Per-timescale scores at EXIT
    exit_score_1min     DOUBLE PRECISION,
    exit_score_5min     DOUBLE PRECISION,
    exit_score_hourly   DOUBLE PRECISION,
    exit_score_daily    DOUBLE PRECISION,
    exit_score_monthly  DOUBLE PRECISION,
    exit_score_composite DOUBLE PRECISION,

    -- Tooling state at entry
    trailing_stop_initial DOUBLE PRECISION,     -- where the trailing stop was set
    take_profit_target    DOUBLE PRECISION,
    max_hold_timeout_ms   BIGINT,

    -- Paper vs live
    is_paper            BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_trades_ticker ON trades(ticker);
CREATE INDEX idx_trades_entry_at ON trades(entry_fill_at DESC);
CREATE INDEX idx_trades_config ON trades(config_version_id);
CREATE INDEX idx_trades_exit_reason ON trades(exit_reason);
CREATE INDEX idx_trades_direction ON trades(direction);

-- Composite index for common agent queries:
-- "show me recent trades where hourly was weak but 5min was strong"
CREATE INDEX idx_trades_score_analysis ON trades(
    entry_score_hourly, entry_score_5min, pnl_percent
);


-- ============================================================================
-- INDICATOR SNAPSHOTS AT TRADE TIME
-- ============================================================================
-- Cached indicator values at entry and exit for each timescale.
-- Avoids expensive recomputation when agents analyze trades.
-- One row per trade per timescale per event (entry/exit).
-- ============================================================================

CREATE TYPE trade_event_type AS ENUM ('entry', 'exit');

CREATE TABLE trade_indicator_snapshots (
    id                  BIGSERIAL PRIMARY KEY,
    trade_id            BIGINT NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
    event_type          trade_event_type NOT NULL,
    timescale           timescale NOT NULL,
    snapshot_at         TIMESTAMPTZ NOT NULL,

    -- Indicator values as JSON since they vary by timescale.
    -- 1min/5min: RSI, MACD, stochastic, bollinger z-score, RoC, etc.
    -- Hourly: VWAP distance, volume ratio, trend slope, etc.
    -- Daily: gap size, daily range position, sector relative strength, etc.
    -- Monthly: VIX level, regime score, broad index trend, etc.
    indicators          JSONB NOT NULL,

    UNIQUE (trade_id, event_type, timescale)
);

CREATE INDEX idx_snapshots_trade ON trade_indicator_snapshots(trade_id);
CREATE INDEX idx_snapshots_timescale ON trade_indicator_snapshots(timescale);


-- ============================================================================
-- AGENT MEMOS
-- ============================================================================
-- Structured outputs from each timescale agent, consumed by the PM agent.
-- Persisted for longitudinal analysis ("has the 5min agent been reporting
-- low confidence for weeks? did performance actually degrade?").
-- ============================================================================

CREATE TABLE agent_memos (
    id                  BIGSERIAL PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    agent               agent_type NOT NULL,
    evolution_cycle_id  BIGINT NOT NULL,        -- groups memos from the same run
    memo_type           memo_type NOT NULL DEFAULT 'recommendation',

    -- Structured fields (PM agent's primary input)
    confidence_score    DOUBLE PRECISION,        -- 0-1, agent's self-assessed confidence
    volatility_regime   VARCHAR(20),             -- 'low', 'normal', 'high', 'extreme'
    directional_bias    VARCHAR(20),             -- 'strong_long', 'lean_long', 'neutral', 'lean_short', 'strong_short'
    signal_quality      VARCHAR(20),             -- 'strong', 'moderate', 'weak', 'conflicting'

    -- Pattern flags (boolean signals the PM agent can key off)
    flags               JSONB NOT NULL DEFAULT '{}',
    -- Examples:
    -- {"divergence_detected": true, "volume_anomaly": true, "level_rejection": false}

    -- Freeform reasoning (goes to RAG store, PM reads for context)
    reasoning           TEXT NOT NULL,

    -- What changes did this agent propose, if any?
    proposed_config_version_id BIGINT REFERENCES config_versions(id),

    -- Performance summary the agent was looking at
    review_period_start TIMESTAMPTZ,
    review_period_end   TIMESTAMPTZ,
    trades_reviewed     INTEGER,
    period_win_rate     DOUBLE PRECISION,
    period_sharpe       DOUBLE PRECISION,
    period_pnl          DOUBLE PRECISION
);

CREATE INDEX idx_memos_agent ON agent_memos(agent);
CREATE INDEX idx_memos_cycle ON agent_memos(evolution_cycle_id);
CREATE INDEX idx_memos_created ON agent_memos(created_at DESC);


-- ============================================================================
-- EVOLUTION CYCLES
-- ============================================================================
-- One row per evolution run. Two types:
--   - full_pm: complete PM-orchestrated cycle (1-3x daily, Sonnet 4.5)
--     All agents run, PM has config authority, changelog written.
--   - checkin: lightweight observation cycle (more frequent, Haiku 4.5)
--     Only fast agents (1min, 5min, optionally hourly) run.
--     They write observation memos but have ZERO config authority.
--     Builds richer evidence base for the next full PM cycle.
-- ============================================================================

CREATE TABLE evolution_cycles (
    id                  BIGSERIAL PRIMARY KEY,
    started_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at        TIMESTAMPTZ,
    trading_date        DATE NOT NULL,           -- which trading day this cycle reviews

    -- Cycle type and model
    cycle_type          cycle_type NOT NULL DEFAULT 'full_pm',
    model_used          VARCHAR(50),             -- e.g., 'claude-sonnet-4-5', 'claude-haiku-4-5'

    -- Which agents ran and their status
    agents_triggered    agent_type[] NOT NULL,
    agents_completed    agent_type[] NOT NULL DEFAULT '{}',

    -- Outcome (only populated for full_pm cycles)
    configs_proposed    INTEGER NOT NULL DEFAULT 0,
    configs_promoted    INTEGER NOT NULL DEFAULT 0,
    configs_rejected    INTEGER NOT NULL DEFAULT 0,

    -- Aggregate trading day stats
    day_total_trades    INTEGER,
    day_total_pnl       DOUBLE PRECISION,
    day_win_rate        DOUBLE PRECISION,
    day_sharpe          DOUBLE PRECISION,

    -- Cost tracking
    input_tokens_used   BIGINT DEFAULT 0,
    output_tokens_used  BIGINT DEFAULT 0,
    estimated_cost_usd  DOUBLE PRECISION DEFAULT 0
);

CREATE INDEX idx_cycles_date ON evolution_cycles(trading_date DESC);


-- ============================================================================
-- CONFIG CHANGELOG
-- ============================================================================
-- Detailed, agent-readable log of every config change.
-- Each row describes ONE atomic change (a single knob turned, a tool
-- enabled/disabled, a weight adjusted). A config version that changes
-- 3 params produces 3 changelog rows.
--
-- Timescale agents read this at the start of every evolution cycle
-- alongside trade data so they can attribute performance shifts to
-- config changes vs. market conditions.
-- ============================================================================

CREATE TYPE change_category AS ENUM (
    'knob_tuned',            -- parameter value changed on existing tool
    'tool_enabled',          -- existing tool instance activated
    'tool_disabled',         -- existing tool instance deactivated
    'tool_instance_added',   -- new instance of existing tool type added
    'tool_instance_removed', -- instance removed from config
    'weight_adjusted',       -- timescale weight or tool weight changed
    'threshold_adjusted',    -- entry/exit threshold changed
    'session_rule_changed',  -- session config changed
    'scoring_changed',       -- aggregation method or hard gates changed
    'new_tool_type_deployed' -- new tool type available after code deploy
);

CREATE TABLE config_changelog (
    id                  BIGSERIAL PRIMARY KEY,
    config_version_id   BIGINT NOT NULL REFERENCES config_versions(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    changed_by          agent_type NOT NULL,

    -- What changed
    change_category     change_category NOT NULL,

    -- Which tool/component was affected
    target_timescale    timescale,              -- NULL if cross-cutting (PM tooling)
    target_tool_id      VARCHAR(100),           -- instance_id of affected tool
    target_tool_type    VARCHAR(100),           -- e.g., "rsi", "trailing_stop"
    target_param        VARCHAR(100),           -- specific param name, if knob_tuned

    -- Values
    old_value           JSONB,                  -- previous value (NULL for new additions)
    new_value           JSONB NOT NULL,         -- new value

    -- Context: why this change was made
    reason              TEXT NOT NULL,           -- agent's reasoning
    evidence_trade_ids  BIGINT[],               -- trades that motivated this change
    evidence_period     TSTZRANGE,              -- time window analyzed

    -- Link to the agent memo that produced this change
    source_memo_id      BIGINT REFERENCES agent_memos(id),

    -- For rollback tracking: if this change reverts a previous change
    reverts_changelog_id BIGINT REFERENCES config_changelog(id)
);

CREATE INDEX idx_changelog_config ON config_changelog(config_version_id);
CREATE INDEX idx_changelog_timescale ON config_changelog(target_timescale);
CREATE INDEX idx_changelog_tool ON config_changelog(target_tool_id);
CREATE INDEX idx_changelog_category ON config_changelog(change_category);
CREATE INDEX idx_changelog_created ON config_changelog(created_at DESC);
CREATE INDEX idx_changelog_agent ON config_changelog(changed_by);

-- View: recent changes relevant to a specific timescale
CREATE VIEW changelog_by_timescale AS
SELECT
    cl.*,
    cv.promoted_at,
    cv.status AS config_status
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status IN ('promoted', 'rolled_back')
ORDER BY cl.created_at DESC;

-- View: changes since a given config version (agent catch-up)
CREATE VIEW changelog_since AS
SELECT
    cl.*,
    cv.promoted_at
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status = 'promoted'
ORDER BY cv.promoted_at ASC, cl.id ASC;


-- ============================================================================
-- CONFIG BLOB SCHEMA REFERENCE
-- ============================================================================
-- The config_blob JSONB in config_versions follows this structure.
-- Documented here as reference; enforced in application code, not DB.
--
-- {
--   "version": "0.1",
--   "last_modified_by": "agent_pm",
--
--   "filters": {
--     "1min": {
--       "enabled": true,
--       "weight": 0.15,
--       "params": {
--         "rsi_period": 14,
--         "rsi_overbought": 72,
--         "rsi_oversold": 28,
--         "volume_spike_threshold": 2.5,
--         "tape_momentum_window": 30
--       }
--     },
--     "5min": {
--       "enabled": true,
--       "weight": 0.25,
--       "params": {
--         "rsi_period": 14,
--         "rsi_overbought": 70,
--         "rsi_oversold": 30,
--         "macd_fast": 12,
--         "macd_slow": 26,
--         "macd_signal": 9,
--         "bollinger_period": 20,
--         "bollinger_std": 2.0
--       }
--     },
--     "1hour": {
--       "enabled": true,
--       "weight": 0.25,
--       "params": {
--         "vwap_distance_threshold": 0.005,
--         "volume_acceleration_window": 5,
--         "trend_slope_period": 10
--       }
--     },
--     "1day": {
--       "enabled": true,
--       "weight": 0.20,
--       "params": {
--         "gap_significance_threshold": 0.005,
--         "daily_range_lookback": 20,
--         "sector_strength_benchmark": "SPY"
--       }
--     },
--     "1month": {
--       "enabled": true,
--       "weight": 0.15,
--       "params": {
--         "vix_regime_thresholds": [15, 20, 30],
--         "trend_ma_period": 50,
--         "regime_lookback_days": 60
--       }
--     }
--   },
--
--   "scoring": {
--     "entry_threshold": 0.65,
--     "exit_threshold": -0.30,
--     "aggregation_method": "weighted_sum"
--   },
--
--   "tooling": {
--     "trailing_stop": {
--       "enabled": true,
--       "initial_offset_pct": 0.003,
--       "tighten_after_profit_pct": 0.005,
--       "tightened_offset_pct": 0.002
--     },
--     "hard_stop": {
--       "enabled": true,
--       "max_loss_pct": 0.01
--     },
--     "take_profit": {
--       "enabled": false,
--       "target_pct": 0.008
--     },
--     "max_hold": {
--       "timeout_minutes": 30,
--       "extend_if_profitable": true,
--       "extended_timeout_minutes": 60
--     },
--     "position_sizing": {
--       "method": "fixed_risk",
--       "risk_per_trade_pct": 0.01,
--       "max_position_pct": 0.05
--     },
--     "session_rules": {
--       "no_new_entries_after": "15:30",
--       "force_exit_by": "15:55",
--       "avoid_first_minutes": 5
--     }
--   },
--
--   "tickers": ["SPY", "QQQ", "AAPL", "NVDA", "MSFT"]
-- }
-- ============================================================================


-- ============================================================================
-- DAILY BUDGET TRACKING
-- ============================================================================
-- The Python orchestrator writes a row per day to enforce spend caps.
-- The orchestrator checks this before starting any new cycle.
-- ============================================================================

CREATE TABLE daily_budget (
    trading_date        DATE PRIMARY KEY,
    total_input_tokens  BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0,
    total_cost_usd      DOUBLE PRECISION NOT NULL DEFAULT 0,
    full_pm_cycles      INTEGER NOT NULL DEFAULT 0,
    checkin_cycles      INTEGER NOT NULL DEFAULT 0,
    budget_limit_usd    DOUBLE PRECISION NOT NULL DEFAULT 5.00,   -- configurable daily cap
    budget_exhausted    BOOLEAN NOT NULL DEFAULT FALSE
);


-- ============================================================================
-- USEFUL VIEWS FOR AGENT QUERIES
-- ============================================================================

-- Daily performance summary (used by most agents)
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
WHERE NOT is_paper OR TRUE  -- include paper trades for now
GROUP BY date_trunc('day', entry_fill_at), ticker, config_version_id;

-- Performance bucketed by exit reason (used by PM agent)
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

-- Score interaction analysis (used by PM agent to understand
-- how timescale combinations affect outcomes)
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

-- Recent agent memo history (used to check for persistent signals)
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

-- Check-in memos since last full PM cycle (PM reads these for context)
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

-- Daily cost summary
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
