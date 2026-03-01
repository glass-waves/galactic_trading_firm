CREATE TABLE config_versions (
    id                  BIGSERIAL PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    status              mutation_status NOT NULL DEFAULT 'proposed',
    promoted_at         TIMESTAMPTZ,
    rolled_back_at      TIMESTAMPTZ,
    created_by          agent_type NOT NULL,
    parent_version_id   BIGINT REFERENCES config_versions(id),
    mutation_reason     TEXT NOT NULL,
    config_blob         JSONB NOT NULL,
    backtest_sharpe     DOUBLE PRECISION,
    backtest_win_rate   DOUBLE PRECISION,
    backtest_total_trades INTEGER,
    backtest_period_start TIMESTAMPTZ,
    backtest_period_end   TIMESTAMPTZ,
    backtest_results    JSONB
);

CREATE INDEX idx_config_status ON config_versions(status);
CREATE INDEX idx_config_created_by ON config_versions(created_by);
CREATE INDEX idx_config_created_at ON config_versions(created_at DESC);

CREATE VIEW active_config AS
SELECT * FROM config_versions
WHERE status = 'promoted'
ORDER BY promoted_at DESC
LIMIT 1;
