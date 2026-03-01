CREATE TABLE agent_memos (
    id                  BIGSERIAL PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    agent               agent_type NOT NULL,
    evolution_cycle_id  BIGINT NOT NULL,
    memo_type           memo_type NOT NULL DEFAULT 'recommendation',
    confidence_score    DOUBLE PRECISION,
    volatility_regime   VARCHAR(20),
    directional_bias    VARCHAR(20),
    signal_quality      VARCHAR(20),
    flags               JSONB NOT NULL DEFAULT '{}',
    reasoning           TEXT NOT NULL,
    proposed_config_version_id BIGINT REFERENCES config_versions(id),
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
