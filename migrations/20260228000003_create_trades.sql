CREATE TABLE trades (
    id                  BIGSERIAL PRIMARY KEY,
    ticker              VARCHAR(10) NOT NULL,
    direction           trade_direction NOT NULL,
    entry_signal_at     TIMESTAMPTZ NOT NULL,
    entry_fill_at       TIMESTAMPTZ NOT NULL,
    exit_signal_at      TIMESTAMPTZ NOT NULL,
    exit_fill_at        TIMESTAMPTZ NOT NULL,
    entry_price         DOUBLE PRECISION NOT NULL,
    exit_price          DOUBLE PRECISION NOT NULL,
    position_size       DOUBLE PRECISION NOT NULL,
    pnl_dollars         DOUBLE PRECISION NOT NULL,
    pnl_percent         DOUBLE PRECISION NOT NULL,
    commission          DOUBLE PRECISION NOT NULL DEFAULT 0,
    slippage_entry      DOUBLE PRECISION,
    slippage_exit       DOUBLE PRECISION,
    hold_duration_ms    BIGINT NOT NULL,
    exit_reason         exit_reason NOT NULL,
    config_version_id   BIGINT NOT NULL REFERENCES config_versions(id),
    entry_score_1min    DOUBLE PRECISION,
    entry_score_5min    DOUBLE PRECISION,
    entry_score_hourly  DOUBLE PRECISION,
    entry_score_daily   DOUBLE PRECISION,
    entry_score_monthly DOUBLE PRECISION,
    entry_score_composite DOUBLE PRECISION NOT NULL,
    exit_score_1min     DOUBLE PRECISION,
    exit_score_5min     DOUBLE PRECISION,
    exit_score_hourly   DOUBLE PRECISION,
    exit_score_daily    DOUBLE PRECISION,
    exit_score_monthly  DOUBLE PRECISION,
    exit_score_composite DOUBLE PRECISION,
    trailing_stop_initial DOUBLE PRECISION,
    take_profit_target    DOUBLE PRECISION,
    max_hold_timeout_ms   BIGINT,
    is_paper            BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_trades_ticker ON trades(ticker);
CREATE INDEX idx_trades_entry_at ON trades(entry_fill_at DESC);
CREATE INDEX idx_trades_config ON trades(config_version_id);
CREATE INDEX idx_trades_exit_reason ON trades(exit_reason);
CREATE INDEX idx_trades_direction ON trades(direction);
CREATE INDEX idx_trades_score_analysis ON trades(
    entry_score_hourly, entry_score_5min, pnl_percent
);
