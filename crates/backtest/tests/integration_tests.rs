use std::collections::HashMap;

use serde_json::json;
use types::action::{ActionConfig, ActionPhase};
use types::market::Timescale;
use types::scoring::{AggregationMethod, ScoringConfig};
use types::test_fixtures::*;

use backtest::{
    compare_configs, comparison_summary, load_candles_from_csv, run_backtest, summary, to_json,
    trades_to_csv, BacktestConfig, BacktestData,
};

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

fn make_backtest_data(ohlcv: Vec<(f64, f64, f64, f64, f64)>) -> BacktestData {
    let timestamps = sequential_timestamps(ohlcv.len());
    let candles: Vec<types::market::Candle> = ohlcv
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

/// full pipeline: config → replay → metrics → JSON → CSV → summary
#[test]
fn full_pipeline_trend_reversal() {
    let config = BacktestConfig {
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
        scoring_config: ScoringConfig {
            timescale_weights: vec![(Timescale::FiveMinute, 1.0)].into_iter().collect(),
            entry_threshold: 0.5,
            exit_threshold: -0.3,
            aggregation: AggregationMethod::WeightedSum,
            hard_gate_timescales: vec![], agreement: None, dynamic_fusion: None,
            hard_gate_indicators: HashMap::new(),
        },
        cost_config: None,
        session_config: None,
    };

    // bullish then bearish data to force entry + exit
    let mut ohlcv = trending_up_ohlcv(40, 100.0, 1.0);
    ohlcv.extend(trending_down_ohlcv(30, 140.0, 2.0));
    let data = make_backtest_data(ohlcv);

    // 1. run backtest
    let result = run_backtest(&config, &data).unwrap();
    assert_eq!(result.ticker, "SPY");
    assert!((result.initial_capital - 10_000.0).abs() < f64::EPSILON);

    // 2. serialize to JSON and verify roundtrip
    let json_str = to_json(&result).unwrap();
    assert!(json_str.contains("\"ticker\""));
    assert!(json_str.contains("\"metrics\""));
    let parsed: backtest::BacktestResult = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.ticker, "SPY");
    assert_eq!(parsed.metrics.total_trades, result.metrics.total_trades);

    // 3. export trades to CSV
    let mut csv_buf = Vec::new();
    trades_to_csv(&result.trades, &mut csv_buf).unwrap();
    let csv_str = String::from_utf8(csv_buf).unwrap();
    assert!(csv_str.contains("ticker"));

    // 4. generate summary
    let text = summary(&result);
    assert!(text.contains("backtest summary"));
    assert!(text.contains("SPY"));
}

/// config comparison: same data, different configs, verify deltas
#[test]
fn config_comparison_on_same_data() {
    let base_config = BacktestConfig {
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
        scoring_config: ScoringConfig {
            timescale_weights: vec![(Timescale::FiveMinute, 1.0)].into_iter().collect(),
            entry_threshold: 0.5,
            exit_threshold: -0.3,
            aggregation: AggregationMethod::WeightedSum,
            hard_gate_timescales: vec![], agreement: None, dynamic_fusion: None,
            hard_gate_indicators: HashMap::new(),
        },
        cost_config: None,
        session_config: None,
    };

    // config B: tighter trailing stop (lower multiplier)
    let mut config_b = base_config.clone();
    config_b.action_configs = vec![
        make_action_cfg("score_threshold_entry", "e1", ActionPhase::Entry, 0,
            vec![("entry_threshold", json!(0.5)), ("short_threshold", json!(-0.5))]),
        make_action_cfg("atr_trailing_stop", "ts1", ActionPhase::Exit, 0,
            vec![("atr_period", json!(14)), ("multiplier", json!(1.0))]), // tighter stop
    ];

    let mut ohlcv = trending_up_ohlcv(40, 100.0, 1.0);
    ohlcv.extend(trending_down_ohlcv(30, 140.0, 2.0));
    let data = make_backtest_data(ohlcv);

    let mut result_a = run_backtest(&base_config, &data).unwrap();
    result_a.config_id = "config_loose_stop".to_string();

    let mut result_b = run_backtest(&config_b, &data).unwrap();
    result_b.config_id = "config_tight_stop".to_string();

    let cmp = compare_configs(&result_a, &result_b);
    assert_eq!(cmp.config_a_id, "config_loose_stop");
    assert_eq!(cmp.config_b_id, "config_tight_stop");

    // the comparison should produce meaningful output
    let cmp_text = comparison_summary(&cmp);
    assert!(cmp_text.contains("config comparison"));
    assert!(cmp_text.contains("config_loose_stop"));
    assert!(cmp_text.contains("config_tight_stop"));

    // comparison should serialize to JSON
    let cmp_json = serde_json::to_string(&cmp).unwrap();
    assert!(cmp_json.contains("pnl_delta"));
}

/// CSV loading → replay pipeline
#[test]
fn csv_to_replay_pipeline() {
    // generate CSV data from trending prices
    let ohlcv = trending_up_ohlcv(30, 100.0, 1.0);
    let timestamps = sequential_timestamps(30);
    let mut csv_data = String::from("timestamp,open,high,low,close,volume\n");
    for (i, &(o, h, l, c, v)) in ohlcv.iter().enumerate() {
        csv_data.push_str(&format!(
            "{},{},{},{},{},{}\n",
            timestamps[i].timestamp(),
            o, h, l, c, v
        ));
    }

    // load from CSV
    let candles = load_candles_from_csv(csv_data.as_bytes()).unwrap();
    assert_eq!(candles.len(), 30);

    // build backtest data from loaded candles
    let mut candle_map = HashMap::new();
    candle_map.insert(Timescale::FiveMinute, candles);
    let data = BacktestData {
        candles: candle_map,
        primary_timescale: Timescale::FiveMinute,
    };

    let config = BacktestConfig {
        ticker: "SPY".to_string(),
        initial_capital: 10_000.0,
        indicator_configs: vec![
            make_indicator_config("rsi", "rsi_5m", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
        ],
        action_configs: vec![
            make_action_cfg("score_threshold_entry", "e1", ActionPhase::Entry, 0,
                vec![("entry_threshold", json!(0.5)), ("short_threshold", json!(-0.5))]),
        ],
        scoring_config: ScoringConfig {
            timescale_weights: vec![(Timescale::FiveMinute, 1.0)].into_iter().collect(),
            entry_threshold: 0.5,
            exit_threshold: -0.3,
            aggregation: AggregationMethod::WeightedSum,
            hard_gate_timescales: vec![], agreement: None, dynamic_fusion: None,
            hard_gate_indicators: HashMap::new(),
        },
        cost_config: None,
        session_config: None,
    };

    let result = run_backtest(&config, &data).unwrap();
    assert_eq!(result.ticker, "SPY");
    // should at least run without error
}

/// verify no panics on edge case: very short data
#[test]
fn very_short_data_does_not_panic() {
    let config = BacktestConfig {
        ticker: "SPY".to_string(),
        initial_capital: 10_000.0,
        indicator_configs: vec![
            make_indicator_config("rsi", "rsi_5m", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]),
        ],
        action_configs: vec![
            make_action_cfg("score_threshold_entry", "e1", ActionPhase::Entry, 0,
                vec![("entry_threshold", json!(0.5))]),
        ],
        scoring_config: ScoringConfig {
            timescale_weights: vec![(Timescale::FiveMinute, 1.0)].into_iter().collect(),
            entry_threshold: 0.5,
            exit_threshold: -0.3,
            aggregation: AggregationMethod::WeightedSum,
            hard_gate_timescales: vec![], agreement: None, dynamic_fusion: None,
            hard_gate_indicators: HashMap::new(),
        },
        cost_config: None,
        session_config: None,
    };

    // only 3 candles — way less than RSI period of 14
    let data = make_backtest_data(trending_up_ohlcv(3, 100.0, 1.0));
    let result = run_backtest(&config, &data).unwrap();
    // should complete without panic, likely no trades
    assert_eq!(result.metrics.total_trades, 0);
}
