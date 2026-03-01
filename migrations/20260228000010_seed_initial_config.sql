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
    'initial seed config for phase 1',
    '{
        "schema_version": "0.1",
        "config_id": 1,
        "created_at": "2026-02-28T00:00:00Z",
        "created_by": "orchestrator",
        "parent_config_id": null,
        "tickers": ["SPY", "QQQ", "AAPL", "NVDA", "MSFT"],
        "indicators": [
            {
                "indicator_type": "rsi",
                "instance_id": "rsi_14_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.3,
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
                "weight": 0.25,
                "params": {"period": 20},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "atr",
                "instance_id": "atr_14_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.2,
                "params": {"period": 14},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "indicator_type": "bollinger",
                "instance_id": "bb_20_5min",
                "timescale": "FiveMinute",
                "enabled": true,
                "weight": 0.25,
                "params": {"period": 20, "std_dev": 2.0},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            }
        ],
        "actions": [
            {
                "action_type": "score_threshold_entry",
                "instance_id": "entry_score",
                "phase": "Entry",
                "enabled": true,
                "priority": 0,
                "params": {},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
            },
            {
                "action_type": "atr_trailing_stop",
                "instance_id": "trailing_stop_atr",
                "phase": "Exit",
                "enabled": true,
                "priority": 0,
                "params": {"atr_multiplier": 2.0, "tighten_after_profit_pct": 0.005},
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
                "params": {"fraction": 0.01},
                "last_modified_by": null,
                "last_modified_at": null,
                "modification_reason": null
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
            }
        ],
        "scoring": {
            "timescale_weights": {"FiveMinute": 1.0},
            "entry_threshold": 0.65,
            "exit_threshold": -0.30,
            "aggregation": "WeightedSum",
            "hard_gate_timescales": []
        },
        "session": {
            "no_new_entries_after": "15:30",
            "force_exit_by": "15:55",
            "avoid_first_minutes": 5,
            "max_concurrent_positions": 3,
            "max_capital_deployed_pct": 0.15
        }
    }'::jsonb
);
