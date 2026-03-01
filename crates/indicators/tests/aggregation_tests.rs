use serde_json::json;
use std::collections::HashMap;
use types::test_fixtures::*;
use types::Timescale;

use indicators::aggregation::{aggregate_timescale_scores, compute_timescale_scores};

#[test]
fn weighted_sum_equal_weights() {
    let pairs = vec![(0.5, 1.0), (-0.3, 1.0), (0.8, 1.0)];
    let result = aggregate_timescale_scores(&pairs).unwrap();
    let expected = (0.5 - 0.3 + 0.8) / 3.0;
    assert!((result - expected).abs() < 1e-10, "got {result}, expected {expected}");
}

#[test]
fn weighted_sum_unequal_weights() {
    let pairs = vec![(1.0, 3.0), (-1.0, 1.0)];
    let result = aggregate_timescale_scores(&pairs).unwrap();
    // (1.0*3.0 + -1.0*1.0) / 4.0 = 2.0 / 4.0 = 0.5
    let expected = 0.5;
    assert!((result - expected).abs() < 1e-10, "got {result}, expected {expected}");
}

#[test]
fn single_indicator_weight_normalizes_to_one() {
    let pairs = vec![(0.7, 5.0)];
    let result = aggregate_timescale_scores(&pairs).unwrap();
    // single indicator: 0.7*5.0 / 5.0 = 0.7
    assert!((result - 0.7).abs() < 1e-10);
}

#[test]
fn no_indicators_returns_none() {
    let pairs: Vec<(f64, f64)> = vec![];
    assert!(aggregate_timescale_scores(&pairs).is_none());
}

#[test]
fn timescale_independence() {
    let configs = vec![
        make_indicator_config("rsi", "rsi_1m", Timescale::OneMinute, 1.0, vec![("period", json!(14))]),
        make_indicator_config("ema", "ema_5m", Timescale::FiveMinute, 1.0, vec![("period", json!(20))]),
    ];

    let mut outputs = HashMap::new();
    outputs.insert("rsi_1m".to_string(), Some(0.8));
    outputs.insert("ema_5m".to_string(), Some(0.3));

    let scores = compute_timescale_scores(&outputs, &configs);
    assert!((scores.one_minute.unwrap() - 0.8).abs() < 1e-10);
    assert!((scores.five_minute.unwrap() - 0.3).abs() < 1e-10);

    // changing 1min shouldn't affect 5min
    outputs.insert("rsi_1m".to_string(), Some(-0.5));
    let scores2 = compute_timescale_scores(&outputs, &configs);
    assert!((scores2.one_minute.unwrap() - (-0.5)).abs() < 1e-10);
    assert!((scores2.five_minute.unwrap() - 0.3).abs() < 1e-10);
}

#[test]
fn indicator_returning_none_excluded_and_weights_renormalized() {
    let configs = vec![
        make_indicator_config("rsi", "rsi_a", Timescale::FiveMinute, 2.0, vec![]),
        make_indicator_config("ema", "ema_b", Timescale::FiveMinute, 1.0, vec![]),
        make_indicator_config("sma", "sma_c", Timescale::FiveMinute, 1.0, vec![]),
    ];

    let mut outputs = HashMap::new();
    outputs.insert("rsi_a".to_string(), Some(0.6));
    outputs.insert("ema_b".to_string(), None); // no data
    outputs.insert("sma_c".to_string(), Some(0.2));

    let scores = compute_timescale_scores(&outputs, &configs);
    // only rsi_a (0.6, w=2.0) and sma_c (0.2, w=1.0) participate
    // weighted sum = (0.6*2.0 + 0.2*1.0) / (2.0+1.0) = 1.4/3.0
    let expected = 1.4 / 3.0;
    assert!(
        (scores.five_minute.unwrap() - expected).abs() < 1e-10,
        "got {:?}, expected {expected}",
        scores.five_minute
    );
}

#[test]
fn full_compute_populates_timescale_scores_fields() {
    let configs = vec![
        make_indicator_config("rsi", "rsi_1m", Timescale::OneMinute, 1.0, vec![]),
        make_indicator_config("ema", "ema_5m", Timescale::FiveMinute, 1.0, vec![]),
        make_indicator_config("sma", "sma_1h", Timescale::OneHour, 1.0, vec![]),
        make_indicator_config("macd", "macd_1d", Timescale::OneDay, 1.0, vec![]),
    ];

    let mut outputs = HashMap::new();
    outputs.insert("rsi_1m".to_string(), Some(0.5));
    outputs.insert("ema_5m".to_string(), Some(-0.2));
    outputs.insert("sma_1h".to_string(), Some(0.7));
    outputs.insert("macd_1d".to_string(), Some(0.1));

    let scores = compute_timescale_scores(&outputs, &configs);
    assert!(scores.one_minute.is_some());
    assert!(scores.five_minute.is_some());
    assert!(scores.one_hour.is_some());
    assert!(scores.one_day.is_some());
    assert!(scores.one_month.is_none()); // no 1month indicator
    assert!((scores.composite - 0.0).abs() < f64::EPSILON); // not set yet
}
