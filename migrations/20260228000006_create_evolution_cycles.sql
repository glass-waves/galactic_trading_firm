CREATE TABLE evolution_cycles (
    id                  BIGSERIAL PRIMARY KEY,
    started_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at        TIMESTAMPTZ,
    trading_date        DATE NOT NULL,
    cycle_type          cycle_type NOT NULL DEFAULT 'full_pm',
    model_used          VARCHAR(50),
    agents_triggered    agent_type[] NOT NULL,
    agents_completed    agent_type[] NOT NULL DEFAULT '{}',
    configs_proposed    INTEGER NOT NULL DEFAULT 0,
    configs_promoted    INTEGER NOT NULL DEFAULT 0,
    configs_rejected    INTEGER NOT NULL DEFAULT 0,
    day_total_trades    INTEGER,
    day_total_pnl       DOUBLE PRECISION,
    day_win_rate        DOUBLE PRECISION,
    day_sharpe          DOUBLE PRECISION,
    input_tokens_used   BIGINT DEFAULT 0,
    output_tokens_used  BIGINT DEFAULT 0,
    estimated_cost_usd  DOUBLE PRECISION DEFAULT 0
);

CREATE INDEX idx_cycles_date ON evolution_cycles(trading_date DESC);
