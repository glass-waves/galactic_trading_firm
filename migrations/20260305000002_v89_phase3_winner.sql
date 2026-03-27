-- NOTE: "v89" was the pre-bug-fix config version number. after fixing the position
-- sizing bug (2026-03-21), config versioning restarted from v1. this migration is
-- historical — see docs/backtest_tuning_log.md for current config lineage.
--
-- v89 config: phase 3 winner — 30s cooldown + adaptive hold + 10% circuit breaker
--
-- key changes from v3 seed:
--   entry_threshold: 0.65 -> 0.58 (tuned over 53 iterations)
--   timescale_weights: 1min 0.20->0.10, 5min 0.50->0.60 (trend-following rebalance)
--   ATR trailing multiplier: 3.0 -> 7.0 (wide trailing, let winners run)
--   fixed stop: 0.025 (2.5%, unchanged)
--   vol-scaled sizing: base_fraction 0.02 -> 0.05
--   session.entry_cooldown_ms: 0 -> 30000 (30s re-entry cooldown)
--   session.max_daily_loss_pct: null -> 0.10 (10% daily loss circuit breaker)
--   max_hold_timeout.profit_extension_ms: 0 -> 1800000 (+30min for winners)
--   max_hold_timeout.loss_reduction_ms: 0 -> 900000 (-15min for losers)
--   5min indicator weights: MACD 0.40, EMA 0.30, StochRSI 0.15, RSI 0.10, BB 0.05
--   1hr indicator weights: SuperTrend 0.30, EMA 0.25, ADX 0.20, VWAP 0.15, BolBW 0.10
--
-- validated post-sizing-fix (4-year backtest, $10k capital, 3.0 bps slippage + $0.005 spread):
--   2022-2025 total P&L: +$1,447 | PF: ~3.1 | trades: 1,847 | win rate: 57%
--   profitable in all regimes: bear (+$536), recovery (+$339), choppy (+$294), recent (+$277)
--   see docs/backtest_tuning_log.md for full results

-- mark old config as superseded
UPDATE config_versions
SET status = 'superseded'
WHERE status = 'promoted';

-- insert v89 config
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
    'config v1: 30s cooldown + adaptive hold (+30m/-15m) + 10% daily loss breaker. validated post-sizing-fix: 4-year PF ~3.1, +$1,447 on $10k',
    '{
    "actions": [
        {
            "phase": "Entry",
            "params": {
                "entry_threshold": 0.58,
                "short_threshold": -1
            },
            "enabled": true,
            "priority": 0,
            "action_type": "score_threshold_entry",
            "instance_id": "entry_score",
            "last_modified_at": "2026-03-03T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "raise entry bar to reduce overtrading (0.45->0.65)"
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
            "modification_reason": "widen trailing stop (2.0x->3.0x ATR)"
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
                "fraction": 0.05
            },
            "enabled": false,
            "priority": 0,
            "action_type": "fixed_fractional",
            "instance_id": "sizing_fixed",
            "last_modified_at": "2026-03-03T00:00:00Z",
            "last_modified_by": "human",
            "modification_reason": "increase sizing so edge covers costs (1%->2%)"
        },
        {
            "phase": "Sizing",
            "params": {
                "lookback": 20,
                "baseline_atr": 1.0,
                "base_fraction": 0.05
            },
            "enabled": true,
            "priority": 0,
            "action_type": "volatility_scaled",
            "instance_id": "sizing_vol",
            "last_modified_at": null,
            "last_modified_by": null,
            "modification_reason": null
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
        "max_capital_deployed_pct": 0.1,
        "max_concurrent_positions": 1
    },
    "tickers": [
        "SPY",
        "QQQ",
        "AAPL",
        "NVDA",
        "MSFT"
    ],
    "config_id": 89,
    "created_at": "2026-03-05T00:00:00Z",
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
    "parent_config_id": 3
}
'::jsonb
);
