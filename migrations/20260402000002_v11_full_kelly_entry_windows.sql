-- v11 config: full kelly sizing + entry windows + candle pattern indicator
--
-- promotes the v10 tuning results (validated across 4 full years, 2022-2025):
--
-- key changes from v1 (v89 seed):
--   entry: score_threshold_entry → entry windows (W1 5m thrust, W4 strong core, W5 candle reversal)
--          + 1m noise reject gate
--   indicators: add candle_pattern on 5min (w=0.05, mean reversion + confluence)
--   sizing: vol-scaled disabled, fixed fractional 0.36 (full kelly, ~36% of capital per trade)
--   session: max_capital_deployed_pct 0.10 → 0.50
--   entry windows: W1 lead_by=0.10, W5 composite_min=0.30 (tuned via sweep)
--
-- 4-year results (SPY/QQQ/AAPL/MSFT, no NVDA, $10k, 2.0 bps slippage + $0.005 spread):
--   baseline (5% sizing, no compound):  +$581 total | PF ~2.75 | 1,124 trades | 55.8% WR
--   full kelly (36% sizing, no compound): +$11,892 total
--   full kelly + compound (day-to-day): +$22,540 total
--
-- compound results per year (full kelly + compound):
--   2022: +$5,340 | 2023: +$5,715 | 2024: +$5,718 | 2025: +$5,767

-- mark old config as superseded
UPDATE config_versions
SET status = 'superseded'
WHERE status = 'promoted';

-- insert v11 config
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
    'v11: full kelly sizing (36%) + entry windows (W1/W4/W5) + candle pattern indicator. 4yr validated: +$22,540 compound on $10k.',
    '{
    "actions": [
        {
            "phase": "Entry",
            "params": {
                "name": "1m noise filter",
                "conditions": [
                    {"type": "timescale_lead", "timescale": "OneMinute", "lead_by": 0.15},
                    {"type": "timescale_max", "timescale": "FiveMinute", "max_score": 0.35}
                ]
            },
            "enabled": true,
            "priority": 0,
            "action_type": "entry_reject_gate",
            "instance_id": "reject_1m_noise",
            "last_modified_at": "2026-04-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "v11: entry windows replace score_threshold_entry"
        },
        {
            "phase": "Entry",
            "params": {
                "name": "5m thrust",
                "direction": "long",
                "conditions": [
                    {"type": "composite_min", "min_score": 0.35},
                    {"type": "timescale_lead", "timescale": "FiveMinute", "lead_by": 0.10},
                    {"type": "timescale_min", "timescale": "FiveMinute", "min_score": 0.50},
                    {"type": "timescale_min", "timescale": "OneHour", "min_score": 0.0}
                ]
            },
            "enabled": true,
            "priority": 10,
            "action_type": "entry_window",
            "instance_id": "window_5m_thrust",
            "last_modified_at": "2026-04-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "v11: W1 5m thrust window, lead_by tuned from 0.15 to 0.10"
        },
        {
            "phase": "Entry",
            "params": {
                "name": "candle reversal",
                "direction": "long",
                "conditions": [
                    {"type": "indicator_min", "instance_id": "candle_5min", "min_score": 0.40},
                    {"type": "timescale_min", "timescale": "OneHour", "min_score": 0.20},
                    {"type": "composite_min", "min_score": 0.30}
                ]
            },
            "enabled": true,
            "priority": 15,
            "action_type": "entry_window",
            "instance_id": "window_candle_reversal",
            "last_modified_at": "2026-04-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "v11: W5 candle reversal window, composite_min tuned to 0.30"
        },
        {
            "phase": "Entry",
            "params": {
                "name": "strong core",
                "direction": "long",
                "conditions": [
                    {"type": "composite_min", "min_score": 0.35},
                    {"type": "timescale_min", "timescale": "FiveMinute", "min_score": 0.40},
                    {"type": "timescale_min", "timescale": "OneHour", "min_score": 0.30}
                ]
            },
            "enabled": true,
            "priority": 20,
            "action_type": "entry_window",
            "instance_id": "window_strong_core",
            "last_modified_at": "2026-04-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "v11: W4 strong core window"
        },
        {
            "phase": "Exit",
            "params": {
                "timescale": "FiveMinute",
                "atr_period": 14,
                "multiplier": 7.0
            },
            "enabled": true,
            "priority": 0,
            "action_type": "atr_trailing_stop",
            "instance_id": "trailing_stop_atr",
            "last_modified_at": "2026-03-03T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "widen trailing stop (2.0x->3.0x->7.0x ATR)"
        },
        {
            "phase": "Exit",
            "params": {
                "stop_loss_pct": 0.025
            },
            "enabled": true,
            "priority": 1,
            "action_type": "fixed_pct_stop",
            "instance_id": "hard_stop",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "phase": "Exit",
            "params": {
                "max_hold_ms": 5400000,
                "loss_reduction_ms": 900000,
                "profit_extension_ms": 1800000
            },
            "enabled": true,
            "priority": 5,
            "action_type": "max_hold_timeout",
            "instance_id": "max_hold",
            "last_modified_at": "2026-03-03T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "extend max hold (45min->90min)"
        },
        {
            "phase": "Exit",
            "params": {
                "force_exit_by": "15:55"
            },
            "enabled": true,
            "priority": 10,
            "action_type": "session_close",
            "instance_id": "exit_session",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "phase": "Monitor",
            "params": {
                "trigger_pct": 0.015,
                "breakeven_trigger_pct": 0.015
            },
            "enabled": true,
            "priority": 0,
            "action_type": "breakeven_stop",
            "instance_id": "breakeven",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "phase": "Sizing",
            "params": {
                "fraction": 0.36
            },
            "enabled": true,
            "priority": 0,
            "action_type": "fixed_fractional",
            "instance_id": "sizing_fixed",
            "last_modified_at": "2026-04-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "full kelly: 36% fixed fractional (vol-scaled disabled — 4yr validated)"
        },
        {
            "phase": "Sizing",
            "params": {
                "lookback": 20,
                "baseline_atr": 1.0,
                "base_fraction": 0.36
            },
            "enabled": false,
            "priority": 0,
            "action_type": "volatility_scaled",
            "instance_id": "sizing_vol",
            "last_modified_at": "2026-04-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "disabled for full kelly — fixed fractional at 36% validated better"
        }
    ],
    "scoring": {
        "agreement": null,
        "aggregation": "WeightedSumWithGates",
        "exit_threshold": -0.15,
        "entry_threshold": 0.58,
        "timescale_weights": {
            "OneHour": 0.3,
            "OneMinute": 0.1,
            "FiveMinute": 0.6
        },
        "hard_gate_timescales": [
            "OneHour"
        ]
    },
    "session": {
        "force_exit_by": "15:55",
        "entry_cooldown_ms": 30000,
        "max_daily_loss_pct": 0.1,
        "avoid_first_minutes": 60,
        "no_new_entries_after": "15:30",
        "max_capital_deployed_pct": 0.50,
        "max_concurrent_positions": 1
    },
    "tickers": [
        "SPY",
        "QQQ",
        "AAPL",
        "NVDA",
        "MSFT"
    ],
    "config_id": 11,
    "created_at": "2026-04-02T00:00:00Z",
    "created_by": "human",
    "indicators": [
        {
            "params": {
                "period": 7,
                "oversold": 30,
                "overbought": 70
            },
            "weight": 0.3,
            "enabled": true,
            "timescale": "OneMinute",
            "instance_id": "rsi_7_1min",
            "indicator_type": "rsi",
            "last_modified_at": "2026-03-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "faster RSI for 1-min timescale"
        },
        {
            "params": {
                "period": 14
            },
            "weight": 0.25,
            "enabled": true,
            "timescale": "OneMinute",
            "instance_id": "stoch_fast_14_1min",
            "indicator_type": "stochastic_fast",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "period": 12
            },
            "weight": 0.2,
            "enabled": true,
            "timescale": "OneMinute",
            "instance_id": "roc_12_1min",
            "indicator_type": "roc",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "fast_period": 6,
                "slow_period": 13,
                "signal_period": 5,
                "normalization_factor": 1.0
            },
            "weight": 0.25,
            "enabled": true,
            "timescale": "OneMinute",
            "instance_id": "macd_fast_1min",
            "indicator_type": "macd",
            "last_modified_at": "2026-03-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "faster MACD for 1-min timescale"
        },
        {
            "params": {
                "period": 14,
                "oversold": 30,
                "overbought": 70
            },
            "weight": 0.1,
            "enabled": true,
            "timescale": "FiveMinute",
            "instance_id": "rsi_14_5min",
            "indicator_type": "rsi",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "period": 20
            },
            "weight": 0.3,
            "enabled": true,
            "timescale": "FiveMinute",
            "instance_id": "ema_20_5min",
            "indicator_type": "ema",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "period": 20,
                "std_dev": 2.0
            },
            "weight": 0.05,
            "enabled": true,
            "timescale": "FiveMinute",
            "instance_id": "bb_20_5min",
            "indicator_type": "bollinger",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "fast_period": 12,
                "slow_period": 26,
                "signal_period": 9,
                "normalization_factor": 1.0
            },
            "weight": 0.4,
            "enabled": true,
            "timescale": "FiveMinute",
            "instance_id": "macd_5min",
            "indicator_type": "macd",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "rsi_period": 14,
                "stoch_period": 14
            },
            "weight": 0.15,
            "enabled": true,
            "timescale": "FiveMinute",
            "instance_id": "stoch_rsi_5min",
            "indicator_type": "stochastic_rsi",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "engulfing_min_body_ratio": 0.5,
                "engulfing_base_score": 0.70,
                "mean_reversion_mode": true,
                "use_confluence": true,
                "affirmative_only": true,
                "volume_lookback": 20,
                "high_volume_threshold": 2.0,
                "low_volume_threshold": 0.5,
                "ema_fast": 5,
                "ema_slow": 20
            },
            "weight": 0.05,
            "enabled": true,
            "timescale": "FiveMinute",
            "instance_id": "candle_5min",
            "indicator_type": "candle_pattern",
            "last_modified_at": "2026-04-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "v11: candle pattern indicator for W5 reversal entries"
        },
        {
            "params": {},
            "weight": 0.15,
            "enabled": true,
            "timescale": "OneHour",
            "instance_id": "vwap_dist_1hr",
            "indicator_type": "vwap_distance",
            "last_modified_at": "2026-03-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "rebalanced for ADX"
        },
        {
            "params": {
                "period": 10,
                "multiplier": 3.0
            },
            "weight": 0.3,
            "enabled": true,
            "timescale": "OneHour",
            "instance_id": "supertrend_1hr",
            "indicator_type": "supertrend",
            "last_modified_at": "2026-03-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "rebalanced for ADX"
        },
        {
            "params": {
                "period": 20
            },
            "weight": 0.25,
            "enabled": true,
            "timescale": "OneHour",
            "instance_id": "ema_20_1hr",
            "indicator_type": "ema",
            "last_modified_at": "2026-03-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "rebalanced for ADX"
        },
        {
            "params": {
                "period": 20,
                "std_dev": 2.0
            },
            "weight": 0.1,
            "enabled": true,
            "timescale": "OneHour",
            "instance_id": "bb_bw_20_1hr",
            "indicator_type": "bollinger_bandwidth",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
        },
        {
            "params": {
                "period": 14
            },
            "weight": 0.2,
            "enabled": true,
            "timescale": "OneHour",
            "instance_id": "adx_14_1hr",
            "indicator_type": "adx",
            "last_modified_at": "2026-03-02T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "ADX trend strength for hourly hard gate"
        }
    ],
    "schema_version": "0.2",
    "parent_config_id": 89
}
'::jsonb
);
