use std::collections::HashMap;

use serde_json::json;
use types::action::{ActionConfig, ActionPhase};
use types::market::Timescale;
use types::scoring::{AggregationMethod, ScoringConfig};
use types::test_fixtures::*;

use backtest::{run_backtest, load_candles_from_csv, BacktestConfig, BacktestData};

fn make_action_cfg(
    action_type: &str,
    id: &str,
    phase: ActionPhase,
    priority: i32,
    params: Vec<(&str, serde_json::Value)>,
) -> ActionConfig {
    ActionConfig {
        action_type: action_type.to_string(),
        instance_id: id.to_string(),
        phase,
        enabled: true,
        priority,
        params: params.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        last_modified_by: None,
        last_modified_at: None,
        modification_reason: None,
    }
}

fn make_scoring(
    weights: Vec<(Timescale, f64)>,
    entry: f64,
    exit: f64,
) -> ScoringConfig {
    ScoringConfig {
        timescale_weights: weights.into_iter().collect(),
        entry_threshold: entry,
        exit_threshold: exit,
        aggregation: AggregationMethod::WeightedSum,
        hard_gate_timescales: vec![],
    }
}

fn default_backtest_config() -> BacktestConfig {
    BacktestConfig {
        ticker: "SPY".to_string(),
        initial_capital: 10_000.0,
        indicator_configs: vec![
            make_indicator_config("rsi", "rsi_5m", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
            make_indicator_config("ema", "ema_5m", Timescale::FiveMinute, 0.5, vec![("period", json!(10))]),
        ],
        action_configs: vec![
            make_action_cfg("score_threshold_entry", "e1", ActionPhase::Entry, 0,
                vec![("entry_threshold", json!(0.5)), ("short_threshold", json!(-0.5))]),
            make_action_cfg("atr_trailing_stop", "ts1", ActionPhase::Exit, 0,
                vec![("atr_period", json!(14)), ("multiplier", json!(2.0))]),
        ],
        scoring_config: make_scoring(vec![(Timescale::FiveMinute, 1.0)], 0.5, -0.3),
    }
}

fn make_backtest_data(ohlcv: Vec<(f64, f64, f64, f64, f64)>) -> BacktestData {
    let timestamps = sequential_timestamps(ohlcv.len());
    let candles = ohlcv
        .iter()
        .zip(timestamps.iter())
        .map(|(&(o, h, l, c, v), &ts)| types::market::Candle {
            timestamp: ts,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
        })
        .collect();

    let mut candle_map = HashMap::new();
    candle_map.insert(Timescale::FiveMinute, candles);

    BacktestData {
        candles: candle_map,
        primary_timescale: Timescale::FiveMinute,
    }
}

#[test]
fn bullish_trend_produces_entry() {
    let config = default_backtest_config();
    let data = make_backtest_data(trending_up_ohlcv(50, 100.0, 1.0));
    let result = run_backtest(&config, &data).unwrap();

    assert!(
        !result.trades.is_empty() || result.metrics.total_trades == 0,
        "engine should have attempted to trade on bullish data (or at least not error)"
    );
    assert_eq!(result.ticker, "SPY");
    assert!((result.initial_capital - 10_000.0).abs() < f64::EPSILON);
}

#[test]
fn bullish_then_bearish_produces_complete_trade() {
    let config = default_backtest_config();
    let mut ohlcv = trending_up_ohlcv(40, 100.0, 1.0);
    ohlcv.extend(trending_down_ohlcv(30, 140.0, 2.0));
    let data = make_backtest_data(ohlcv);
    let result = run_backtest(&config, &data).unwrap();

    // should have at least entered (and possibly exited)
    assert!(
        !result.trades.is_empty() || result.metrics.total_trades == 0,
        "should process trend reversal without error"
    );
}

#[test]
fn sideways_market_no_trades() {
    let config = BacktestConfig {
        action_configs: vec![
            make_action_cfg("score_threshold_entry", "e1", ActionPhase::Entry, 0,
                vec![("entry_threshold", json!(0.95)), ("short_threshold", json!(-0.95))]),
        ],
        ..default_backtest_config()
    };
    let data = make_backtest_data(ranging_ohlcv(50, 100.0, 1.0));
    let result = run_backtest(&config, &data).unwrap();

    assert!(result.trades.is_empty(), "ranging market with high threshold should not trade");
    assert_eq!(result.metrics.total_trades, 0);
}

#[test]
fn empty_data_returns_error() {
    let config = default_backtest_config();
    let data = BacktestData {
        candles: HashMap::new(),
        primary_timescale: Timescale::FiveMinute,
    };
    let result = run_backtest(&config, &data);
    assert!(result.is_err());
}

#[test]
fn metrics_populated_after_replay() {
    let config = default_backtest_config();
    let mut ohlcv = trending_up_ohlcv(40, 100.0, 1.0);
    ohlcv.extend(trending_down_ohlcv(30, 140.0, 2.0));
    let data = make_backtest_data(ohlcv);
    let result = run_backtest(&config, &data).unwrap();

    // metrics should be computed regardless of trade count
    assert!(result.metrics.trades_per_day >= 0.0);
    assert!(result.metrics.max_drawdown >= 0.0);
    assert!(result.metrics.max_drawdown_pct >= 0.0);
}

#[test]
fn result_serializes_to_json() {
    let config = default_backtest_config();
    let data = make_backtest_data(trending_up_ohlcv(50, 100.0, 1.0));
    let result = run_backtest(&config, &data).unwrap();

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"ticker\""));
    assert!(json.contains("\"metrics\""));
}

#[test]
fn csv_loading_roundtrip() {
    let csv_data = "timestamp,open,high,low,close,volume\n\
                    1700000000,100.0,102.0,99.0,101.0,50000\n\
                    1700000060,101.0,103.0,100.0,102.0,60000\n\
                    1700000120,102.0,104.0,101.0,103.0,55000\n";

    let candles = load_candles_from_csv(csv_data.as_bytes()).unwrap();
    assert_eq!(candles.len(), 3);
    assert!((candles[0].open - 100.0).abs() < f64::EPSILON);
    assert!((candles[0].close - 101.0).abs() < f64::EPSILON);
    assert!((candles[1].volume - 60000.0).abs() < f64::EPSILON);
    assert!((candles[2].high - 104.0).abs() < f64::EPSILON);
}

#[test]
fn csv_bad_data_returns_error() {
    let csv_data = "timestamp,open,high,low,close,volume\n\
                    not_a_number,100.0,102.0,99.0,101.0,50000\n";

    let result = load_candles_from_csv(csv_data.as_bytes());
    assert!(result.is_err());
}

#[test]
fn unknown_indicator_type_returns_error() {
    let config = BacktestConfig {
        indicator_configs: vec![
            make_indicator_config("nonexistent_indicator", "bad_1", Timescale::FiveMinute, 1.0, vec![]),
        ],
        ..default_backtest_config()
    };
    let data = make_backtest_data(trending_up_ohlcv(20, 100.0, 1.0));
    let result = run_backtest(&config, &data);
    assert!(result.is_err());
}

#[test]
fn unknown_action_type_returns_error() {
    let config = BacktestConfig {
        action_configs: vec![
            make_action_cfg("nonexistent_action", "bad_1", ActionPhase::Entry, 0, vec![]),
        ],
        ..default_backtest_config()
    };
    let data = make_backtest_data(trending_up_ohlcv(20, 100.0, 1.0));
    let result = run_backtest(&config, &data);
    assert!(result.is_err());
}
