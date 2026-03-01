use serde_json::json;
use std::collections::HashMap;
use types::test_fixtures::*;
use types::Timescale;

use indicators::aggregation::compute_timescale_scores;
use indicators::{build_indicators, default_indicator_registry};

#[test]
fn full_pipeline_config_to_timescale_scores() {
    let reg = default_indicator_registry();
    let configs = vec![
        make_indicator_config("rsi", "rsi_5m", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
        make_indicator_config("ema", "ema_5m", Timescale::FiveMinute, 0.5, vec![("period", json!(10))]),
        make_indicator_config("atr", "atr_1m", Timescale::OneMinute, 1.0, vec![("period", json!(14))]),
    ];

    let indicators = build_indicators(&configs, &reg).unwrap();
    assert_eq!(indicators.len(), 3);

    // build market state with both timescales
    let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 0.5));
    ms.candles.insert(
        Timescale::OneMinute,
        make_market_state_ohlcv(Timescale::OneMinute, &trending_up_ohlcv(30, 100.0, 0.3))
            .candles
            .remove(&Timescale::OneMinute)
            .unwrap(),
    );

    // compute all indicator outputs
    let mut outputs: HashMap<String, Option<f64>> = HashMap::new();
    for (id, ind) in &indicators {
        let score = ind.compute(&ms).map(|o| o.score);
        outputs.insert(id.clone(), score);
    }

    // compute timescale scores
    let scores = compute_timescale_scores(&outputs, &configs);

    // 5min should be populated (RSI + EMA)
    assert!(scores.five_minute.is_some(), "five_minute should have a score");
    // 1min should be populated (ATR)
    assert!(scores.one_minute.is_some(), "one_minute should have a score");
    // others should be None
    assert!(scores.one_hour.is_none());
    assert!(scores.one_day.is_none());
    assert!(scores.one_month.is_none());
}

#[test]
fn every_registered_indicator_computes_without_panic() {
    let reg = default_indicator_registry();
    let all_types: Vec<&str> = vec![
        "rsi", "ema", "sma", "macd", "bollinger", "atr", "keltner",
        "stochastic_fast", "stochastic_slow", "cci", "mfi", "roc", "obv",
        "bollinger_pct_b", "bollinger_bandwidth", "adx", "supertrend",
        "vwap_distance", "stochastic_rsi", "williams_r", "donchian", "dema",
        "ttm_squeeze", "awesome_oscillator",
    ];

    let ms = make_market_state_ohlcv(
        Timescale::FiveMinute,
        &trending_up_ohlcv(100, 100.0, 0.3),
    );

    for (i, typ) in all_types.iter().enumerate() {
        let cfg = make_indicator_config(
            typ,
            &format!("{typ}_{i}"),
            Timescale::FiveMinute,
            1.0,
            vec![],
        );
        let factory = reg.factories.get(*typ).unwrap_or_else(|| panic!("missing factory for '{typ}'"));
        let ind = factory(&cfg);

        // should not panic — either returns Some or None
        let result = ind.compute(&ms);
        if let Some(ref output) = result {
            assert!(
                output.score >= -1.0 && output.score <= 1.0,
                "{typ} score out of range: {}",
                output.score
            );
        }
    }
}

#[test]
fn bad_indicator_params_handled_gracefully() {
    let reg = default_indicator_registry();

    // period=0 should either return None or handle gracefully, never panic
    let configs = vec![
        make_indicator_config("rsi", "rsi_bad", Timescale::FiveMinute, 1.0, vec![("period", json!(0))]),
        make_indicator_config("ema", "ema_bad", Timescale::FiveMinute, 1.0, vec![("period", json!(0))]),
        make_indicator_config("bollinger", "bb_bad", Timescale::FiveMinute, 1.0, vec![("period", json!(0))]),
    ];

    let indicators = build_indicators(&configs, &reg).unwrap();
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));

    for (id, ind) in &indicators {
        // catch_unwind to verify no panics even with bad params
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ind.compute(&ms)
        }));
        assert!(result.is_ok(), "indicator '{id}' panicked with bad params");
    }
}
