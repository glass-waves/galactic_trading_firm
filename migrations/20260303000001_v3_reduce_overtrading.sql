-- v3 config: reduce overtrading for cost-adjusted profitability
--
-- key changes:
--   entry_threshold 0.45 -> 0.65 (fewer, higher-conviction entries)
--   short_threshold -0.45 -> -0.65
--   atr trailing stop multiplier 2.0 -> 3.0 (wider stops, longer holds)
--   max_hold_ms 2700000 -> 5400000 (45 min -> 90 min)
--   avoid_first_minutes 5 -> 15 (skip opening noise)
--   sizing fraction 0.01 -> 0.02 (bigger positions to justify cost)
--
-- rationale: with realistic costs (~5 bps round trip), the v2 config's
-- 70-300 trades/day churns through capital. raising the entry bar and
-- widening stops should cut trade count by 50-70% while letting winners
-- run long enough to overcome transaction costs.

-- mark old config as superseded
UPDATE config_versions
SET status = 'superseded'
WHERE status = 'promoted';

-- insert v3 config
INSERT INTO config_versions (
    status,
    promoted_at,
    created_by,
    mutation_reason,
    config_blob
) VALUES (
    'promoted',
    now(),
    'orchestrator',
    'v3 config — reduce overtrading: raise entry threshold 0.45->0.65, widen ATR stop 2x->3x, extend max hold 45->90min, skip first 15min, increase sizing 1%->2%',
    '{
        "schema_version": "0.2",
        "config_id": 3,
        "created_at": "2026-03-03T00:00:00Z",
        "created_by": "human",
        "parent_config_id": 2,
        "tickers": ["SPY", "QQQ", "AAPL", "NVDA", "MSFT"],

        "indicators": [
            {
                "indicator_type": "rsi",
                "instance_id": "rsi_7_1min",
                "timescale": "OneMinute",
                "enabled": true,
                "weight": 0.30,
                "params": {"period": 7, "overbought": 70, "oversold": 30},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-02T00:00:00Z",
                "modification_reason": "faster RSI for 1-min timescale (period 14 -> 7)"
            },
            {
                "indicator_type": "stochastic_fast",
                "instance_id": "stoch_fast_14_1min",
                "timescale": "OneMinute",
                "enabled": true,
                "weight": 0.25,
                "params": {"period": 14},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "roc",
                "instance_id": "roc_12_1min",
                "timescale": "OneMinute",
                "enabled": true,
                "weight": 0.20,
                "params": {"period": 12},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "macd",
                "instance_id": "macd_fast_1min",
                "timescale": "OneMinute",
                "enabled": true,
                "weight": 0.25,
                "params": {"fast_period": 6, "slow_period": 13, "signal_period": 5, "normalization_factor": 1.0},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-02T00:00:00Z",
                "modification_reason": "faster MACD for 1-min timescale (12/26/9 -> 6/13/5)"
            },

            {
                "indicator_type": "rsi",
                "instance_id": "rsi_14_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.25,
                "params": {"period": 14, "overbought": 70, "oversold": 30},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "ema",
                "instance_id": "ema_20_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.15,
                "params": {"period": 20},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "bollinger",
                "instance_id": "bb_20_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.15,
                "params": {"period": 20, "std_dev": 2.0},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "macd",
                "instance_id": "macd_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.25,
                "params": {"fast_period": 12, "slow_period": 26, "signal_period": 9, "normalization_factor": 1.0},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "stochastic_rsi",
                "instance_id": "stoch_rsi_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.20,
                "params": {"rsi_period": 14, "stoch_period": 14},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },

            {
                "indicator_type": "vwap_distance",
                "instance_id": "vwap_dist_1hr",
                "timescale": "OneHour",
                "enabled": true,
                "weight": 0.25,
                "params": {},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-02T00:00:00Z",
                "modification_reason": "rebalanced to accommodate ADX (0.30 -> 0.25)"
            },
            {
                "indicator_type": "supertrend",
                "instance_id": "supertrend_1hr",
                "timescale": "OneHour",
                "enabled": true,
                "weight": 0.25,
                "params": {"period": 10, "multiplier": 3.0},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-02T00:00:00Z",
                "modification_reason": "rebalanced to accommodate ADX (0.30 -> 0.25)"
            },
            {
                "indicator_type": "ema",
                "instance_id": "ema_20_1hr",
                "timescale": "OneHour",
                "enabled": true,
                "weight": 0.20,
                "params": {"period": 20},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-02T00:00:00Z",
                "modification_reason": "rebalanced to accommodate ADX (0.25 -> 0.20)"
            },
            {
                "indicator_type": "bollinger_bandwidth",
                "instance_id": "bb_bw_20_1hr",
                "timescale": "OneHour",
                "enabled": true,
                "weight": 0.15,
                "params": {"period": 20, "std_dev": 2.0},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "adx",
                "instance_id": "adx_14_1hr",
                "timescale": "OneHour",
                "enabled": true,
                "weight": 0.15,
                "params": {"period": 14},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-02T00:00:00Z",
                "modification_reason": "new: ADX trend strength for hourly hard gate timescale"
            }
        ],

        "actions": [
            {
                "action_type": "score_threshold_entry",
                "instance_id": "entry_score",
                "phase": "Entry",
                "enabled": true,
                "priority": 0,
                "params": {"entry_threshold": 0.65, "short_threshold": -0.65},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-03T00:00:00Z",
                "modification_reason": "raise entry bar to reduce overtrading (0.45 -> 0.65)"
            },
            {
                "action_type": "atr_trailing_stop",
                "instance_id": "trailing_stop_atr",
                "phase": "Exit",
                "enabled": true,
                "priority": 0,
                "params": {"atr_period": 14, "multiplier": 3.0, "timescale": "FiveMinute"},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-03T00:00:00Z",
                "modification_reason": "widen trailing stop to let winners run (2.0x -> 3.0x ATR)"
            },
            {
                "action_type": "fixed_pct_stop",
                "instance_id": "hard_stop",
                "phase": "Exit",
                "enabled": true,
                "priority": 1,
                "params": {"stop_loss_pct": 0.015},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "action_type": "max_hold_timeout",
                "instance_id": "max_hold",
                "phase": "Exit",
                "enabled": true,
                "priority": 5,
                "params": {"max_hold_ms": 5400000},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-03T00:00:00Z",
                "modification_reason": "extend max hold to let trades develop (45min -> 90min)"
            },
            {
                "action_type": "session_close",
                "instance_id": "exit_session",
                "phase": "Exit",
                "enabled": true,
                "priority": 10,
                "params": {"force_exit_by": "15:55"},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "action_type": "breakeven_stop",
                "instance_id": "breakeven",
                "phase": "Monitor",
                "enabled": true,
                "priority": 0,
                "params": {"trigger_pct": 0.008},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "action_type": "fixed_fractional",
                "instance_id": "sizing_fixed",
                "phase": "Sizing",
                "enabled": true,
                "priority": 0,
                "params": {"fraction": 0.02},
                "last_modified_by": "human",
                "last_modified_at": "2026-03-03T00:00:00Z",
                "modification_reason": "increase position size so edge covers costs (1% -> 2%)"
            }
        ],

        "scoring": {
            "timescale_weights": {
                "OneMinute": 0.20,
                "FiveMinute": 0.50,
                "OneHour": 0.30
            },
            "entry_threshold": 0.65,
            "exit_threshold": -0.15,
            "aggregation": "WeightedSumWithGates",
            "hard_gate_timescales": ["OneHour"]
        },

        "session": {
            "no_new_entries_after": "15:30",
            "force_exit_by": "15:55",
            "avoid_first_minutes": 15,
            "max_concurrent_positions": 2,
            "max_capital_deployed_pct": 0.10
        }
    }'::jsonb
);
