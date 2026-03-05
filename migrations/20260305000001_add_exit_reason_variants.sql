-- add new exit_reason enum values for score-based exit and daily loss circuit breaker
ALTER TYPE exit_reason ADD VALUE IF NOT EXISTS 'score_exit';
ALTER TYPE exit_reason ADD VALUE IF NOT EXISTS 'daily_loss_limit';
