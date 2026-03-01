use types::action::{ActionConfig, ActionPhase};
use types::registry::ActionRegistry;

use actions::{build_actions, default_action_registry};

fn make_action_config(
    action_type: &str,
    instance_id: &str,
    phase: ActionPhase,
    priority: i32,
) -> ActionConfig {
    ActionConfig {
        action_type: action_type.to_string(),
        instance_id: instance_id.to_string(),
        phase,
        enabled: true,
        priority,
        params: Default::default(),
        last_modified_by: None,
        last_modified_at: None,
        modification_reason: None,
    }
}

#[test]
fn registry_register_and_create() {
    let mut reg = ActionRegistry::new();
    reg.register("score_threshold_entry", actions::entry::score_threshold::score_threshold_entry_factory);
    let cfg = make_action_config("score_threshold_entry", "e1", ActionPhase::Entry, 0);
    let factory = reg.factories.get("score_threshold_entry").expect("factory should exist");
    let action = factory(&cfg);
    assert_eq!(action.name(), "score_threshold_entry");
}

#[test]
fn registry_unknown_type_returns_error() {
    let reg = default_action_registry();
    let cfg = make_action_config("nonexistent_action", "bad", ActionPhase::Entry, 0);
    let result = build_actions(&[cfg], &reg);
    assert!(result.is_err());
}

#[test]
fn registry_actions_sorted_by_priority() {
    let reg = default_action_registry();
    let configs = vec![
        make_action_config("atr_trailing_stop", "ts_low", ActionPhase::Exit, 10),
        make_action_config("fixed_pct_stop", "fps_high", ActionPhase::Exit, 0),
        make_action_config("session_close", "sc_mid", ActionPhase::Exit, 5),
    ];
    let result = build_actions(&configs, &reg).unwrap();
    assert_eq!(result.exit.len(), 3);
    assert_eq!(result.exit[0].name(), "fixed_pct_stop");       // priority 0
    assert_eq!(result.exit[1].name(), "session_close");         // priority 5
    assert_eq!(result.exit[2].name(), "atr_trailing_stop");     // priority 10
}

#[test]
fn registry_disabled_actions_not_loaded() {
    let reg = default_action_registry();
    let mut cfg = make_action_config("score_threshold_entry", "e1", ActionPhase::Entry, 0);
    cfg.enabled = false;
    let result = build_actions(&[cfg], &reg).unwrap();
    assert!(result.entry.is_empty());
}

#[test]
fn registry_actions_grouped_by_phase() {
    let reg = default_action_registry();
    let configs = vec![
        make_action_config("score_threshold_entry", "e1", ActionPhase::Entry, 0),
        make_action_config("breakeven_stop", "be1", ActionPhase::Monitor, 0),
        make_action_config("atr_trailing_stop", "ts1", ActionPhase::Exit, 0),
        make_action_config("fixed_fractional", "ff1", ActionPhase::Sizing, 0),
    ];
    let result = build_actions(&configs, &reg).unwrap();
    assert_eq!(result.entry.len(), 1);
    assert_eq!(result.monitor.len(), 1);
    assert_eq!(result.exit.len(), 1);
    assert_eq!(result.sizing.len(), 1);
}

#[test]
fn default_registry_has_all_types() {
    let reg = default_action_registry();
    let expected = vec![
        "score_threshold_entry", "atr_trailing_stop", "fixed_pct_stop",
        "session_close", "max_hold_timeout", "breakeven_stop", "fixed_fractional",
    ];
    for t in expected {
        assert!(reg.factories.contains_key(t), "registry missing factory for '{t}'");
    }
}
