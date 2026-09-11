-- observability for live paper trading + enum values for claude agents.
--
-- before this migration the only write path from the engine to postgres was
-- one row per COMPLETED trade. an outside observer (dashboard, watchdog,
-- intraday tuning agent) could not tell "quiet market" from "dead websocket"
-- from "misconfigured gate vetoing everything". see docs/paper_trading_plan_2026-09.md.

-- 1. live per-ticker engine state. upserted on every bar and every 30s heartbeat.
CREATE TABLE engine_state (
    ticker                      TEXT PRIMARY KEY,
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_bar_at                 TIMESTAMPTZ,
    last_price                  DOUBLE PRECISION,
    composite                   DOUBLE PRECISION,
    score_1min                  DOUBLE PRECISION,
    score_5min                  DOUBLE PRECISION,
    score_hourly                DOUBLE PRECISION,
    position_direction          TEXT,
    position_entry_price        DOUBLE PRECISION,
    position_size               DOUBLE PRECISION,
    position_unrealized_pnl     DOUBLE PRECISION,
    position_unrealized_pnl_pct DOUBLE PRECISION,
    position_hold_ms            BIGINT,
    position_opened_at          TIMESTAMPTZ,
    position_entry_reason       TEXT,
    daily_pnl                   DOUBLE PRECISION NOT NULL DEFAULT 0,
    ticker_realized_pnl         DOUBLE PRECISION NOT NULL DEFAULT 0,
    loss_breaker_active         BOOLEAN NOT NULL DEFAULT false,
    entry_blocked_by            TEXT,
    near_miss                   TEXT,
    feed_stale                  BOOLEAN NOT NULL DEFAULT false,
    config_version_id           BIGINT REFERENCES config_versions(id),
    pending_config_version_id   BIGINT,
    process_started_at          TIMESTAMPTZ,
    broker_mode                 TEXT
);

-- 2. why entries were NOT taken. append-only, throttled by the writer.
--    kind = 'gate'      → a session gate or reject gate blocked evaluation
--                         (reason: avoid_first_minutes | no_new_entries_after | entry_cooldown |
--                          daily_loss_breaker | entries_blocked | reject_gate:<name>)
--    kind = 'near_miss' → windows were evaluated, at least one met its composite floor,
--                         none fired (reason lists the failing conditions per window)
CREATE TABLE entry_block_events (
    id                  BIGSERIAL PRIMARY KEY,
    ts                  TIMESTAMPTZ NOT NULL,
    ticker              TEXT NOT NULL,
    kind                TEXT NOT NULL,
    reason              TEXT NOT NULL,
    composite           DOUBLE PRECISION,
    score_1min          DOUBLE PRECISION,
    score_5min          DOUBLE PRECISION,
    score_hourly        DOUBLE PRECISION,
    last_price          DOUBLE PRECISION,
    config_version_id   BIGINT
);
CREATE INDEX idx_entry_block_events_ts ON entry_block_events(ts DESC);
CREATE INDEX idx_entry_block_events_ticker_ts ON entry_block_events(ticker, ts DESC);

-- 3. trade attribution: which entry window fired, where the row came from, broker fills.
ALTER TABLE trades
    ADD COLUMN entry_reason        TEXT,
    ADD COLUMN source              TEXT NOT NULL DEFAULT 'paper',
    ADD COLUMN broker_entry_price  DOUBLE PRECISION,
    ADD COLUMN broker_exit_price   DOUBLE PRECISION;
CREATE INDEX idx_trades_source_exit ON trades(source, exit_fill_at DESC);

-- 4. honest provenance for configs and memos written by humans and claude agents.
ALTER TYPE agent_type ADD VALUE IF NOT EXISTS 'human';
ALTER TYPE agent_type ADD VALUE IF NOT EXISTS 'claude_intraday';
ALTER TYPE agent_type ADD VALUE IF NOT EXISTS 'claude_eod';
ALTER TYPE memo_type ADD VALUE IF NOT EXISTS 'watchdog_warning';
ALTER TYPE memo_type ADD VALUE IF NOT EXISTS 'watchdog_critical';
ALTER TYPE memo_type ADD VALUE IF NOT EXISTS 'intraday_review';
ALTER TYPE memo_type ADD VALUE IF NOT EXISTS 'eod_review';

-- memos no longer belong to an orchestrator "evolution cycle"
ALTER TABLE agent_memos ALTER COLUMN evolution_cycle_id DROP NOT NULL;
