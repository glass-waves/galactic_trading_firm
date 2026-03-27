use std::collections::HashMap;

use serde_json::json;
use types::action::{Action, ActionPhase, ActionSignal, TradeDirection};
use types::market::Timescale;
use types::scoring::TimescaleScores;
use types::test_fixtures::*;

use actions::entry::conditions::{all_conditions_met, parse_conditions, WindowCondition};
use actions::entry::entry_reject_gate::EntryRejectGateAction;
use actions::entry::entry_window::EntryWindowAction;

fn scores(one_min: f64, five_min: f64, one_hr: f64) -> TimescaleScores {
    TimescaleScores {
        one_minute: Some(one_min),
        five_minute: Some(five_min),
        one_hour: Some(one_hr),
        ..Default::default()
    }
}

fn scores_with_indicators(
    one_min: f64,
    five_min: f64,
    one_hr: f64,
    indicators: Vec<(&str, f64)>,
) -> TimescaleScores {
    let mut ind_map: HashMap<String, Option<f64>> = HashMap::new();
    for (id, score) in indicators {
        ind_map.insert(id.to_string(), Some(score));
    }
    TimescaleScores {
        one_minute: Some(one_min),
        five_minute: Some(five_min),
        one_hour: Some(one_hr),
        indicator_scores: Some(ind_map),
        ..Default::default()
    }
}

// ============================================================
// condition engine tests
// ============================================================

#[test]
fn timescale_min_passes() {
    let cond = WindowCondition::TimescaleMin {
        timescale: Timescale::FiveMinute,
        min_score: 0.50,
    };
    let s = scores(0.3, 0.55, 0.2);
    assert!(cond.evaluate(&s, None));
}

#[test]
fn timescale_min_fails() {
    let cond = WindowCondition::TimescaleMin {
        timescale: Timescale::FiveMinute,
        min_score: 0.50,
    };
    let s = scores(0.3, 0.45, 0.2);
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn timescale_min_missing_fails() {
    let cond = WindowCondition::TimescaleMin {
        timescale: Timescale::OneDay,
        min_score: 0.10,
    };
    let s = scores(0.3, 0.5, 0.2); // no one_day
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn timescale_max_passes() {
    let cond = WindowCondition::TimescaleMax {
        timescale: Timescale::OneMinute,
        max_score: 0.35,
    };
    let s = scores(0.30, 0.5, 0.2);
    assert!(cond.evaluate(&s, None));
}

#[test]
fn timescale_max_fails() {
    let cond = WindowCondition::TimescaleMax {
        timescale: Timescale::OneMinute,
        max_score: 0.25,
    };
    let s = scores(0.30, 0.5, 0.2);
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn timescale_range_passes() {
    let cond = WindowCondition::TimescaleRange {
        timescale: Timescale::FiveMinute,
        min_score: -0.10,
        max_score: 0.20,
    };
    let s = scores(0.5, 0.15, 0.8);
    assert!(cond.evaluate(&s, None));
}

#[test]
fn timescale_range_fails() {
    let cond = WindowCondition::TimescaleRange {
        timescale: Timescale::FiveMinute,
        min_score: -0.10,
        max_score: 0.20,
    };
    let s = scores(0.5, 0.35, 0.8);
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn timescale_lead_passes() {
    let cond = WindowCondition::TimescaleLead {
        timescale: Timescale::FiveMinute,
        lead_by: 0.15,
    };
    // 5m=0.70 leads 1m=0.30 by 0.40 and 1h=0.40 by 0.30 — both >= 0.15
    let s = scores(0.30, 0.70, 0.40);
    assert!(cond.evaluate(&s, None));
}

#[test]
fn timescale_lead_fails() {
    let cond = WindowCondition::TimescaleLead {
        timescale: Timescale::FiveMinute,
        lead_by: 0.15,
    };
    // 5m=0.50 leads 1m=0.30 by 0.20 but only leads 1h=0.40 by 0.10 — fails
    let s = scores(0.30, 0.50, 0.40);
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn timescale_spread_max_passes() {
    let cond = WindowCondition::TimescaleSpreadMax { max_spread: 0.15 };
    let s = scores(0.40, 0.45, 0.50); // spread = 0.10
    assert!(cond.evaluate(&s, None));
}

#[test]
fn timescale_spread_max_fails() {
    let cond = WindowCondition::TimescaleSpreadMax { max_spread: 0.15 };
    let s = scores(0.20, 0.45, 0.50); // spread = 0.30
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn timescale_all_min_passes() {
    let cond = WindowCondition::TimescaleAllMin { min_score: 0.10 };
    let s = scores(0.15, 0.20, 0.30);
    assert!(cond.evaluate(&s, None));
}

#[test]
fn timescale_all_min_fails() {
    let cond = WindowCondition::TimescaleAllMin { min_score: 0.10 };
    let s = scores(0.05, 0.20, 0.30); // 1m=0.05 < 0.10
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn indicator_min_passes() {
    let cond = WindowCondition::IndicatorMin {
        instance_id: "macd_5min".to_string(),
        min_score: 0.30,
    };
    let s = scores_with_indicators(0.3, 0.5, 0.2, vec![("macd_5min", 0.45)]);
    assert!(cond.evaluate(&s, s.indicator_scores.as_ref()));
}

#[test]
fn indicator_min_fails() {
    let cond = WindowCondition::IndicatorMin {
        instance_id: "macd_5min".to_string(),
        min_score: 0.30,
    };
    let s = scores_with_indicators(0.3, 0.5, 0.2, vec![("macd_5min", 0.20)]);
    assert!(!cond.evaluate(&s, s.indicator_scores.as_ref()));
}

#[test]
fn indicator_min_missing_fails() {
    let cond = WindowCondition::IndicatorMin {
        instance_id: "nonexistent".to_string(),
        min_score: 0.30,
    };
    let s = scores_with_indicators(0.3, 0.5, 0.2, vec![("macd_5min", 0.45)]);
    assert!(!cond.evaluate(&s, s.indicator_scores.as_ref()));
}

#[test]
fn indicator_max_passes() {
    let cond = WindowCondition::IndicatorMax {
        instance_id: "adx_1hr".to_string(),
        max_score: 0.25,
    };
    let s = scores_with_indicators(0.3, 0.5, 0.2, vec![("adx_1hr", 0.20)]);
    assert!(cond.evaluate(&s, s.indicator_scores.as_ref()));
}

#[test]
fn indicator_range_passes() {
    let cond = WindowCondition::IndicatorRange {
        instance_id: "rsi_5min".to_string(),
        min_score: 0.20,
        max_score: 0.60,
    };
    let s = scores_with_indicators(0.3, 0.5, 0.2, vec![("rsi_5min", 0.40)]);
    assert!(cond.evaluate(&s, s.indicator_scores.as_ref()));
}

#[test]
fn composite_min_passes() {
    let cond = WindowCondition::CompositeMin { min_score: 0.30 };
    let mut s = scores(0.3, 0.5, 0.2);
    s.composite = 0.35;
    assert!(cond.evaluate(&s, None));
}

#[test]
fn composite_min_fails() {
    let cond = WindowCondition::CompositeMin { min_score: 0.30 };
    let mut s = scores(0.3, 0.5, 0.2);
    s.composite = 0.25;
    assert!(!cond.evaluate(&s, None));
}

#[test]
fn all_conditions_and_logic() {
    let conditions = vec![
        WindowCondition::TimescaleMin {
            timescale: Timescale::FiveMinute,
            min_score: 0.40,
        },
        WindowCondition::TimescaleMin {
            timescale: Timescale::OneHour,
            min_score: 0.10,
        },
    ];
    // both pass
    let s1 = scores(0.3, 0.50, 0.20);
    assert!(all_conditions_met(&conditions, &s1, None));

    // 5m fails
    let s2 = scores(0.3, 0.35, 0.20);
    assert!(!all_conditions_met(&conditions, &s2, None));

    // 1h fails
    let s3 = scores(0.3, 0.50, 0.05);
    assert!(!all_conditions_met(&conditions, &s3, None));
}

#[test]
fn condition_from_json_parsing() {
    let json_conditions = json!([
        {"type": "timescale_min", "timescale": "5m", "min_score": 0.50},
        {"type": "timescale_lead", "timescale": "FiveMinute", "lead_by": 0.15},
        {"type": "indicator_min", "instance_id": "macd_5min", "min_score": 0.30},
        {"type": "timescale_spread_max", "max_spread": 0.15},
        {"type": "timescale_all_min", "min_score": 0.10},
        {"type": "timescale_range", "timescale": "5m", "min_score": -0.10, "max_score": 0.20}
    ]);
    let conditions = parse_conditions(&json_conditions).unwrap();
    assert_eq!(conditions.len(), 6);
}

#[test]
fn condition_from_json_unknown_type_errors() {
    let json_conditions = json!([
        {"type": "nonexistent_condition", "value": 0.5}
    ]);
    assert!(parse_conditions(&json_conditions).is_err());
}

// ============================================================
// window action tests
// ============================================================

#[test]
fn window_enters_when_conditions_met() {
    let window = EntryWindowAction::new(
        "test_window".to_string(),
        vec![
            WindowCondition::TimescaleMin {
                timescale: Timescale::FiveMinute,
                min_score: 0.40,
            },
            WindowCondition::TimescaleMin {
                timescale: Timescale::OneHour,
                min_score: 0.0,
            },
        ],
        TradeDirection::Long,
    );
    let s = scores(0.3, 0.55, 0.20);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    match window.evaluate(None, &ms, &s) {
        ActionSignal::Enter { direction, reason, .. } => {
            assert_eq!(direction, TradeDirection::Long);
            assert!(reason.contains("test_window"));
        }
        other => panic!("expected Enter, got {:?}", other),
    }
}

#[test]
fn window_holds_when_conditions_not_met() {
    let window = EntryWindowAction::new(
        "test_window".to_string(),
        vec![WindowCondition::TimescaleMin {
            timescale: Timescale::FiveMinute,
            min_score: 0.60,
        }],
        TradeDirection::Long,
    );
    let s = scores(0.3, 0.45, 0.20);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(window.evaluate(None, &ms, &s), ActionSignal::Hold));
}

#[test]
fn window_holds_when_in_position() {
    let window = EntryWindowAction::new(
        "test_window".to_string(),
        vec![WindowCondition::TimescaleMin {
            timescale: Timescale::FiveMinute,
            min_score: 0.10,
        }],
        TradeDirection::Long,
    );
    let pos = types::action::Position {
        ticker: "SPY".to_string(),
        direction: TradeDirection::Long,
        entry_price: 100.0,
        current_price: 101.0,
        size: 10.0,
        entry_time: chrono::Utc::now(),
        unrealized_pnl: 10.0,
        unrealized_pnl_pct: 0.01,
        high_water_mark: 101.0,
        low_water_mark: 100.0,
        hold_duration_ms: 60_000,
    };
    let s = scores(0.5, 0.5, 0.5);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(
        window.evaluate(Some(&pos), &ms, &s),
        ActionSignal::Hold
    ));
}

#[test]
fn window_with_indicator_condition() {
    let window = EntryWindowAction::new(
        "ofi_window".to_string(),
        vec![
            WindowCondition::IndicatorMin {
                instance_id: "ofi_5min".to_string(),
                min_score: 0.50,
            },
            WindowCondition::TimescaleMin {
                timescale: Timescale::OneHour,
                min_score: 0.0,
            },
        ],
        TradeDirection::Long,
    );
    let s = scores_with_indicators(0.3, 0.4, 0.2, vec![("ofi_5min", 0.60)]);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(
        window.evaluate(None, &ms, &s),
        ActionSignal::Enter { .. }
    ));
}

#[test]
fn window_reason_includes_name() {
    let window = EntryWindowAction::new(
        "5m_thrust".to_string(),
        vec![], // empty conditions = always enters
        TradeDirection::Long,
    );
    let s = scores(0.0, 0.0, 0.0);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    match window.evaluate(None, &ms, &s) {
        ActionSignal::Enter { reason, .. } => {
            assert_eq!(reason, "window:5m_thrust");
        }
        other => panic!("expected Enter, got {:?}", other),
    }
}

#[test]
fn window_phase_is_entry() {
    let window = EntryWindowAction::new("w".to_string(), vec![], TradeDirection::Long);
    assert_eq!(window.phase(), ActionPhase::Entry);
}

#[test]
fn five_min_thrust_scenario() {
    // realistic 5m thrust: 5m=0.65, 1m=0.30, 1h=0.35
    let window = EntryWindowAction::new(
        "5m_thrust".to_string(),
        vec![
            WindowCondition::TimescaleLead {
                timescale: Timescale::FiveMinute,
                lead_by: 0.15,
            },
            WindowCondition::TimescaleMin {
                timescale: Timescale::FiveMinute,
                min_score: 0.50,
            },
            WindowCondition::TimescaleMin {
                timescale: Timescale::OneHour,
                min_score: 0.0,
            },
        ],
        TradeDirection::Long,
    );
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);

    // strong 5m thrust — should enter
    let s1 = scores(0.30, 0.65, 0.35);
    assert!(matches!(
        window.evaluate(None, &ms, &s1),
        ActionSignal::Enter { .. }
    ));

    // 5m strong but doesn't lead 1h by 0.15 — should hold
    let s2 = scores(0.30, 0.55, 0.45);
    assert!(matches!(window.evaluate(None, &ms, &s2), ActionSignal::Hold));

    // 5m leads but below 0.50 — should hold
    let s3 = scores(0.20, 0.45, 0.25);
    assert!(matches!(window.evaluate(None, &ms, &s3), ActionSignal::Hold));
}

#[test]
fn aligned_bias_scenario() {
    let window = EntryWindowAction::new(
        "aligned".to_string(),
        vec![
            WindowCondition::TimescaleSpreadMax { max_spread: 0.15 },
            WindowCondition::TimescaleAllMin { min_score: 0.10 },
        ],
        TradeDirection::Long,
    );
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);

    // aligned: 0.35, 0.40, 0.45 — spread=0.10, all>0.10
    let s1 = scores(0.35, 0.40, 0.45);
    assert!(matches!(
        window.evaluate(None, &ms, &s1),
        ActionSignal::Enter { .. }
    ));

    // not aligned: 0.10, 0.40, 0.50 — spread=0.40
    let s2 = scores(0.10, 0.40, 0.50);
    assert!(matches!(window.evaluate(None, &ms, &s2), ActionSignal::Hold));

    // aligned but one too low: 0.05, 0.10, 0.15 — all_min fails
    let s3 = scores(0.05, 0.10, 0.15);
    assert!(matches!(window.evaluate(None, &ms, &s3), ActionSignal::Hold));
}

// ============================================================
// reject gate tests
// ============================================================

#[test]
fn gate_rejects_when_conditions_met() {
    let gate = EntryRejectGateAction::new(
        "1m_noise".to_string(),
        vec![
            WindowCondition::TimescaleLead {
                timescale: Timescale::OneMinute,
                lead_by: 0.15,
            },
            WindowCondition::TimescaleMax {
                timescale: Timescale::FiveMinute,
                max_score: 0.35,
            },
        ],
    );
    // 1m=0.70 leads 5m=0.30 by 0.40 and 1h=0.20 by 0.50 — both >= 0.15. 5m=0.30 <= 0.35.
    let s = scores(0.70, 0.30, 0.20);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(
        gate.evaluate(None, &ms, &s),
        ActionSignal::RejectEntry
    ));
}

#[test]
fn gate_holds_when_conditions_not_met() {
    let gate = EntryRejectGateAction::new(
        "1m_noise".to_string(),
        vec![
            WindowCondition::TimescaleLead {
                timescale: Timescale::OneMinute,
                lead_by: 0.15,
            },
            WindowCondition::TimescaleMax {
                timescale: Timescale::FiveMinute,
                max_score: 0.35,
            },
        ],
    );
    // 5m=0.55 is strong, 1m=0.40 doesn't lead — gate should NOT fire
    let s = scores(0.40, 0.55, 0.30);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(gate.evaluate(None, &ms, &s), ActionSignal::Hold));
}

#[test]
fn gate_holds_when_in_position() {
    let gate = EntryRejectGateAction::new(
        "gate".to_string(),
        vec![WindowCondition::TimescaleAllMin { min_score: -1.0 }], // always true
    );
    let pos = types::action::Position {
        ticker: "SPY".to_string(),
        direction: TradeDirection::Long,
        entry_price: 100.0,
        current_price: 101.0,
        size: 10.0,
        entry_time: chrono::Utc::now(),
        unrealized_pnl: 10.0,
        unrealized_pnl_pct: 0.01,
        high_water_mark: 101.0,
        low_water_mark: 100.0,
        hold_duration_ms: 60_000,
    };
    let s = scores(0.5, 0.5, 0.5);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(
        gate.evaluate(Some(&pos), &ms, &s),
        ActionSignal::Hold
    ));
}

#[test]
fn gate_phase_is_entry() {
    let gate = EntryRejectGateAction::new("g".to_string(), vec![]);
    assert_eq!(gate.phase(), ActionPhase::Entry);
}

#[test]
fn one_min_noise_scenario() {
    let gate = EntryRejectGateAction::new(
        "1m_noise_filter".to_string(),
        vec![
            WindowCondition::TimescaleLead {
                timescale: Timescale::OneMinute,
                lead_by: 0.15,
            },
            WindowCondition::TimescaleMax {
                timescale: Timescale::FiveMinute,
                max_score: 0.35,
            },
        ],
    );
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);

    // noisy 1m-led entry: 1m=0.80, 5m=0.30, 1h=0.20 — should reject
    let s1 = scores(0.80, 0.30, 0.20);
    assert!(matches!(
        gate.evaluate(None, &ms, &s1),
        ActionSignal::RejectEntry
    ));

    // healthy entry: 1m=0.40, 5m=0.55, 1h=0.30 — should pass
    let s2 = scores(0.40, 0.55, 0.30);
    assert!(matches!(gate.evaluate(None, &ms, &s2), ActionSignal::Hold));

    // 1m leads but 5m is strong: 1m=0.70, 5m=0.50, 1h=0.30 — should pass (5m > 0.35)
    let s3 = scores(0.70, 0.50, 0.30);
    assert!(matches!(gate.evaluate(None, &ms, &s3), ActionSignal::Hold));
}

// ============================================================
// factory tests
// ============================================================

#[test]
fn window_factory_parses_config() {
    use types::action::ActionConfig;

    let config = ActionConfig {
        action_type: "entry_window".to_string(),
        instance_id: "window_5m_thrust".to_string(),
        phase: ActionPhase::Entry,
        enabled: true,
        priority: 10,
        params: serde_json::from_value(json!({
            "name": "5m thrust",
            "direction": "long",
            "conditions": [
                {"type": "timescale_lead", "timescale": "FiveMinute", "lead_by": 0.15},
                {"type": "timescale_min", "timescale": "FiveMinute", "min_score": 0.50}
            ]
        }))
        .unwrap(),
        last_modified_by: None,
        last_modified_at: None,
        modification_reason: None,
    };

    let action = actions::entry::entry_window::entry_window_factory(&config);
    assert_eq!(action.name(), "5m thrust");
    assert_eq!(action.phase(), ActionPhase::Entry);

    // should enter with matching scores
    let s = scores(0.20, 0.65, 0.30);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(
        action.evaluate(None, &ms, &s),
        ActionSignal::Enter { .. }
    ));
}

#[test]
fn reject_gate_factory_parses_config() {
    use types::action::ActionConfig;

    let config = ActionConfig {
        action_type: "entry_reject_gate".to_string(),
        instance_id: "reject_1m_noise".to_string(),
        phase: ActionPhase::Entry,
        enabled: true,
        priority: 0,
        params: serde_json::from_value(json!({
            "name": "1m noise filter",
            "conditions": [
                {"type": "timescale_lead", "timescale": "OneMinute", "lead_by": 0.15},
                {"type": "timescale_max", "timescale": "FiveMinute", "max_score": 0.35}
            ]
        }))
        .unwrap(),
        last_modified_by: None,
        last_modified_at: None,
        modification_reason: None,
    };

    let action = actions::entry::entry_reject_gate::entry_reject_gate_factory(&config);
    assert_eq!(action.name(), "1m noise filter");

    // noisy entry — should reject
    let s = scores(0.80, 0.30, 0.20);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 20]);
    assert!(matches!(
        action.evaluate(None, &ms, &s),
        ActionSignal::RejectEntry
    ));
}
