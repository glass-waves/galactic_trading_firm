-- v13: symmetric (long + short) morning windows, QQQ removed, score exit -0.30, 15 % sizing.
--
-- evidence (corrected replay, 2026-09-11; see docs/paper_trading_plan_2026-09.md):
--   v12 long-only:            2026 +$1,450 (PF 1.28)   2025 -$3,369 (PF 0.60)
--   + no QQQ, exit -0.30:     2026 +$2,065 (PF 1.51)   2025 -$2,758 (PF 0.62)
--   + mirrored short windows: 2026 +$2,743 (PF 1.42)   2025 -$1,932 (PF 0.82)
-- each step is better than the previous in BOTH years. the short book is positive in
-- both years (+$648 / +$704); the long book is a bet on positive morning drift.
--
-- mechanics:
--   * QQQ was the only losing ticker in 2026 (PF 0.54) and adds nothing the four
--     mega-caps don't already cover.
--   * score exit at -0.15 was the entire loss book (132 trades, 13 % win); at -0.30 it
--     fires ~11 times a year and the max-hold rule does the exiting.
--   * the hourly hard gate floors the composite at 0 when the hourly score is negative,
--     which makes short windows unreachable; every window carries its own hourly
--     condition, so the gate is removed. the long book is unchanged by this (+$2,095
--     vs +$2,065 on 2026).
--   * short twins mirror every long threshold: min<->max with the sign flipped,
--     lead<->lag. the candle-reversal window has no symmetric meaning and is not mirrored.
--   * sizing 0.36 -> 0.15: kelly on the honest 2026 numbers is ~0.17 (47 % win, 1.77
--     payoff); 2025 is negative expectancy. max_position_pct clamps to the same value.
--
-- the engine's score exit is direction-aware as of this migration's commit; the
-- alpaca paper account has shorting enabled.

INSERT INTO config_versions (status, promoted_at, created_by, parent_version_id, mutation_reason, config_blob)
SELECT
    'promoted',
    now(),
    'human',
    cv.id,
    'v13: mirrored short windows (hard gate removed), QQQ dropped, score exit -0.30, sizing 0.15. corrected-replay evidence: better than v12 in both 2026 (+$2,743 vs +$1,450) and 2025 (-$1,932 vs -$3,369).',
    (
        cv.config_blob
        || jsonb_build_object(
            'config_id', 13,
            'parent_config_id', cv.config_blob->'config_id',
            'created_at', to_jsonb(now()),
            'created_by', 'human',
            'tickers', '["AMZN", "AAPL", "NVDA", "MSFT"]'::jsonb,
            'scoring', (cv.config_blob->'scoring') || '{"exit_threshold": -0.30, "hard_gate_timescales": []}'::jsonb,
            'session', (cv.config_blob->'session') || '{"max_position_pct": 0.15}'::jsonb
        )
        || jsonb_build_object(
            'actions',
            (
                SELECT jsonb_agg(
                    CASE WHEN a->>'instance_id' = 'sizing_fixed'
                         THEN jsonb_set(a, '{params,fraction}', '0.15'::jsonb)
                         ELSE a END
                )
                FROM jsonb_array_elements(cv.config_blob->'actions') a
            )
            || '[
                {
                    "phase": "Entry",
                    "action_type": "entry_reject_gate",
                    "instance_id": "reject_1m_noise_short",
                    "enabled": true,
                    "priority": 0,
                    "params": {
                        "name": "1m noise filter short",
                        "conditions": [
                            {"type": "timescale_lag", "lag_by": 0.15, "timescale": "OneMinute"},
                            {"type": "timescale_min", "min_score": -0.35, "timescale": "FiveMinute"}
                        ]
                    },
                    "last_modified_at": "2026-09-12T00:00:00Z",
                    "last_modified_by": "human",
                    "modification_reason": "v13: mirror of reject_1m_noise for short entries"
                },
                {
                    "phase": "Entry",
                    "action_type": "entry_window",
                    "instance_id": "window_5m_thrust_short",
                    "enabled": true,
                    "priority": 11,
                    "params": {
                        "name": "5m thrust short",
                        "direction": "short",
                        "conditions": [
                            {"type": "composite_max", "max_score": -0.35},
                            {"type": "timescale_lag", "lag_by": 0.10, "timescale": "FiveMinute"},
                            {"type": "timescale_max", "max_score": -0.50, "timescale": "FiveMinute"},
                            {"type": "timescale_max", "max_score": 0.0, "timescale": "OneHour"}
                        ]
                    },
                    "last_modified_at": "2026-09-12T00:00:00Z",
                    "last_modified_by": "human",
                    "modification_reason": "v13: mirror of window_5m_thrust"
                },
                {
                    "phase": "Entry",
                    "action_type": "entry_window",
                    "instance_id": "window_strong_core_short",
                    "enabled": true,
                    "priority": 21,
                    "params": {
                        "name": "strong core short",
                        "direction": "short",
                        "conditions": [
                            {"type": "composite_max", "max_score": -0.35},
                            {"type": "timescale_max", "max_score": -0.40, "timescale": "FiveMinute"},
                            {"type": "timescale_max", "max_score": -0.30, "timescale": "OneHour"}
                        ]
                    },
                    "last_modified_at": "2026-09-12T00:00:00Z",
                    "last_modified_by": "human",
                    "modification_reason": "v13: mirror of window_strong_core"
                }
            ]'::jsonb
        )
    )
FROM config_versions cv
WHERE cv.status = 'promoted'
ORDER BY cv.id DESC
LIMIT 1;

UPDATE config_versions
SET status = 'superseded'
WHERE status = 'promoted'
  AND id < (SELECT max(id) FROM config_versions WHERE status = 'promoted');
