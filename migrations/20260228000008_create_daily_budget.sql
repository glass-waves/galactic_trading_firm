CREATE TABLE daily_budget (
    trading_date        DATE PRIMARY KEY,
    total_input_tokens  BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0,
    total_cost_usd      DOUBLE PRECISION NOT NULL DEFAULT 0,
    full_pm_cycles      INTEGER NOT NULL DEFAULT 0,
    checkin_cycles      INTEGER NOT NULL DEFAULT 0,
    budget_limit_usd    DOUBLE PRECISION NOT NULL DEFAULT 5.00,
    budget_exhausted    BOOLEAN NOT NULL DEFAULT FALSE
);
