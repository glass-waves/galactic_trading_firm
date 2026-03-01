use serde_json::json;
use types::test_fixtures::*;
use types::Timescale;

use indicators::{build_indicators, default_indicator_registry};
use types::registry::IndicatorRegistry;

#[test]
fn registry_register_and_create() {
    let mut reg = IndicatorRegistry::new();
    reg.register("rsi", indicators::native::rsi::rsi_factory);

    let cfg = make_indicator_config("rsi", "rsi_14", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]);
    let factory = reg.factories.get("rsi").expect("factory should exist");
    let ind = factory(&cfg);
    assert_eq!(ind.name(), "rsi");
}

#[test]
fn registry_unknown_type_returns_error() {
    let reg = default_indicator_registry();
    let cfg = make_indicator_config(
        "nonexistent_indicator",
        "bad_1",
        Timescale::FiveMinute,
        1.0,
        vec![],
    );
    let result = build_indicators(&[cfg], &reg);
    assert!(result.is_err());
}

#[test]
fn registry_build_from_config_multiple_indicators() {
    let reg = default_indicator_registry();
    let configs = vec![
        make_indicator_config("rsi", "rsi_14", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
        make_indicator_config("ema", "ema_20", Timescale::FiveMinute, 0.5, vec![("period", json!(20))]),
        make_indicator_config("atr", "atr_14", Timescale::FiveMinute, 0.3, vec![("period", json!(14))]),
    ];
    let indicators = build_indicators(&configs, &reg).unwrap();
    assert_eq!(indicators.len(), 3);
    assert!(indicators.contains_key("rsi_14"));
    assert!(indicators.contains_key("ema_20"));
    assert!(indicators.contains_key("atr_14"));
}

#[test]
fn registry_disabled_indicators_not_loaded() {
    let reg = default_indicator_registry();
    let mut cfg = make_indicator_config("rsi", "rsi_off", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]);
    cfg.enabled = false;
    let indicators = build_indicators(&[cfg], &reg).unwrap();
    assert!(indicators.is_empty());
}

#[test]
fn registry_multiple_instances_of_same_type() {
    let reg = default_indicator_registry();
    let configs = vec![
        make_indicator_config("rsi", "rsi_7", Timescale::OneMinute, 1.0, vec![("period", json!(7))]),
        make_indicator_config("rsi", "rsi_14", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
        make_indicator_config("rsi", "rsi_21", Timescale::OneHour, 1.0, vec![("period", json!(21))]),
    ];
    let indicators = build_indicators(&configs, &reg).unwrap();
    assert_eq!(indicators.len(), 3);
    assert_eq!(indicators["rsi_7"].timescale(), Timescale::OneMinute);
    assert_eq!(indicators["rsi_14"].timescale(), Timescale::FiveMinute);
    assert_eq!(indicators["rsi_21"].timescale(), Timescale::OneHour);
}

#[test]
fn default_registry_has_all_native_types() {
    let reg = default_indicator_registry();
    let expected_types = vec![
        "rsi", "ema", "sma", "macd", "bollinger", "atr", "keltner",
        "stochastic_fast", "stochastic_slow", "cci", "mfi", "roc", "obv",
    ];
    for t in expected_types {
        assert!(
            reg.factories.contains_key(t),
            "registry missing factory for '{t}'"
        );
    }
}
