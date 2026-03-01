"""tests for config diff computation."""

from agents.tools.config_diff import compute_config_diff


def _base_config():
    return {
        "tickers": ["SPY", "QQQ"],
        "scoring": {
            "entry_threshold": 0.65,
            "exit_threshold": -0.30,
            "aggregation": "WeightedSum",
            "timescale_weights": {"FiveMinute": 0.5, "OneMinute": 0.3, "Hourly": 0.2},
            "hard_gate_timescales": ["OneMinute"],
        },
        "session": {
            "no_new_entries_after": "15:30",
            "force_exit_by": "15:55",
            "avoid_first_minutes": 5,
            "max_concurrent_positions": 3,
        },
        "indicators": [
            {
                "indicator_type": "rsi",
                "instance_id": "rsi_14",
                "timescale": "FiveMinute",
                "enabled": True,
                "weight": 1.0,
                "params": {"period": 14},
            },
            {
                "indicator_type": "ema",
                "instance_id": "ema_20",
                "timescale": "FiveMinute",
                "enabled": True,
                "weight": 0.8,
                "params": {"period": 20},
            },
        ],
        "actions": [
            {
                "action_type": "score_threshold_entry",
                "instance_id": "entry_1",
                "enabled": True,
                "params": {},
            },
        ],
    }


def test_no_changes():
    config = _base_config()
    changes = compute_config_diff(config, config)
    assert changes == []


def test_knob_tuned():
    old = _base_config()
    new = _base_config()
    new["indicators"][0]["params"]["period"] = 21
    changes = compute_config_diff(old, new)
    assert len(changes) == 1
    assert changes[0]["change_category"] == "knob_tuned"
    assert changes[0]["target_param"] == "period"
    assert changes[0]["old_value"] == 14
    assert changes[0]["new_value"] == 21
    assert changes[0]["target_tool_id"] == "rsi_14"


def test_tool_enabled():
    old = _base_config()
    new = _base_config()
    old["indicators"][1]["enabled"] = False
    new["indicators"][1]["enabled"] = True
    changes = compute_config_diff(old, new)
    enabled_changes = [c for c in changes if c["target_tool_id"] == "ema_20" and c["target_param"] == "enabled"]
    assert len(enabled_changes) == 1
    assert enabled_changes[0]["change_category"] == "tool_enabled"


def test_tool_disabled():
    old = _base_config()
    new = _base_config()
    old["indicators"][1]["enabled"] = True
    new["indicators"][1]["enabled"] = False
    changes = compute_config_diff(old, new)
    disabled_changes = [c for c in changes if c["target_tool_id"] == "ema_20" and c["target_param"] == "enabled"]
    assert len(disabled_changes) == 1
    assert disabled_changes[0]["change_category"] == "tool_disabled"


def test_weight_adjusted():
    old = _base_config()
    new = _base_config()
    new["scoring"]["timescale_weights"]["FiveMinute"] = 0.6
    changes = compute_config_diff(old, new)
    weight_changes = [c for c in changes if c["change_category"] == "weight_adjusted"]
    assert len(weight_changes) == 1
    assert weight_changes[0]["target_timescale"] == "FiveMinute"
    assert weight_changes[0]["old_value"] == 0.5
    assert weight_changes[0]["new_value"] == 0.6


def test_threshold_adjusted():
    old = _base_config()
    new = _base_config()
    new["scoring"]["entry_threshold"] = 0.70
    changes = compute_config_diff(old, new)
    threshold_changes = [c for c in changes if c["change_category"] == "threshold_adjusted"]
    assert len(threshold_changes) == 1
    assert threshold_changes[0]["target_param"] == "entry_threshold"
    assert threshold_changes[0]["old_value"] == 0.65
    assert threshold_changes[0]["new_value"] == 0.70


def test_multiple_changes():
    old = _base_config()
    new = _base_config()
    new["indicators"][0]["params"]["period"] = 21
    new["scoring"]["entry_threshold"] = 0.70
    new["session"]["avoid_first_minutes"] = 10
    changes = compute_config_diff(old, new)
    assert len(changes) == 3
    categories = {c["change_category"] for c in changes}
    assert "knob_tuned" in categories
    assert "threshold_adjusted" in categories
    assert "session_rule_changed" in categories


def test_nested_param_change():
    old = _base_config()
    new = _base_config()
    new["indicators"][0]["params"]["upper_band"] = 2.5
    changes = compute_config_diff(old, new)
    param_changes = [c for c in changes if c["target_param"] == "upper_band"]
    assert len(param_changes) == 1
    assert param_changes[0]["change_category"] == "knob_tuned"
    assert param_changes[0]["old_value"] is None
    assert param_changes[0]["new_value"] == 2.5
