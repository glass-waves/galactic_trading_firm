use chrono::{TimeZone, Utc};
use engine::TradeRecord;
use types::action::{ExitReason, TradeDirection};

use backtest::{
    compare_configs, comparison_summary, compute_metrics, summary, to_json, trades_to_csv,
    BacktestResult,
};

fn make_trade(
    pnl: f64,
    pnl_pct: f64,
    exit_reason: ExitReason,
    hold_ms: i64,
    day_offset: i64,
) -> TradeRecord {
    let entry_time = Utc.timestamp_opt(1_700_000_000 + day_offset * 86400, 0).unwrap();
    let exit_time = Utc
        .timestamp_opt(1_700_000_000 + day_offset * 86400 + hold_ms / 1000, 0)
        .unwrap();

    TradeRecord {
        ticker: "SPY".to_string(),
        direction: TradeDirection::Long,
        entry_price: 100.0,
        exit_price: 100.0 + pnl / 100.0,
        size: 100.0,
        entry_time,
        exit_time,
        pnl,
        pnl_pct,
        hold_duration_ms: hold_ms,
        exit_reason,
        high_water_mark: 102.0,
        low_water_mark: 99.0,
    }
}

fn default_period() -> (chrono::DateTime<Utc>, chrono::DateTime<Utc>) {
    let start = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let end = Utc.timestamp_opt(1_700_000_000 + 10 * 86400, 0).unwrap();
    (start, end)
}

fn make_result(
    config_id: &str,
    trades: Vec<TradeRecord>,
) -> BacktestResult {
    let (start, end) = default_period();
    let (metrics, equity_curve) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    BacktestResult {
        config_id: config_id.to_string(),
        ticker: "SPY".to_string(),
        start_time: start,
        end_time: end,
        initial_capital: 10_000.0,
        trades,
        trade_scores: vec![],
        equity_curve,
        metrics,
        max_composite: 0.0,
        max_composite_time: None,
        positive_score_ticks: 0,
        total_ticks: 0,
    }
}

#[test]
fn json_roundtrip() {
    let result = make_result(
        "v1",
        vec![
            make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
            make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        ],
    );
    let json = to_json(&result).unwrap();
    let parsed: BacktestResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.config_id, "v1");
    assert_eq!(parsed.metrics.total_trades, 2);
    assert!((parsed.metrics.total_pnl - 30.0).abs() < f64::EPSILON);
}

#[test]
fn csv_export_has_correct_columns() {
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
    ];
    let mut buf = Vec::new();
    trades_to_csv(&trades, &mut buf).unwrap();
    let csv_str = String::from_utf8(buf).unwrap();

    // check header
    let lines: Vec<&str> = csv_str.lines().collect();
    assert!(lines[0].contains("ticker"));
    assert!(lines[0].contains("direction"));
    assert!(lines[0].contains("entry_price"));
    assert!(lines[0].contains("exit_price"));
    assert!(lines[0].contains("pnl"));
    assert!(lines[0].contains("exit_reason"));

    // check data rows
    assert_eq!(lines.len(), 3); // header + 2 trades
    assert!(lines[1].contains("SPY"));
    assert!(lines[1].contains("Long"));
    assert!(lines[1].contains("TrailingStop"));
}

#[test]
fn csv_export_empty_trades() {
    let mut buf = Vec::new();
    trades_to_csv(&[], &mut buf).unwrap();
    let csv_str = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = csv_str.lines().collect();
    assert_eq!(lines.len(), 1); // header only
}

#[test]
fn summary_contains_key_fields() {
    let result = make_result(
        "test_v1",
        vec![
            make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
            make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        ],
    );
    let text = summary(&result);
    assert!(text.contains("backtest summary"));
    assert!(text.contains("SPY"));
    assert!(text.contains("test_v1"));
    assert!(text.contains("total P&L"));
    assert!(text.contains("win rate"));
    assert!(text.contains("sharpe (daily)"));
    assert!(text.contains("max drawdown"));
    assert!(text.contains("TrailingStop"));
    assert!(text.contains("HardStop"));
}

#[test]
fn config_comparison_deltas_correct() {
    let result_a = make_result(
        "v1",
        vec![
            make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
            make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        ],
    );
    let result_b = make_result(
        "v2",
        vec![
            make_trade(80.0, 0.008, ExitReason::TrailingStop, 60_000, 0),
            make_trade(10.0, 0.001, ExitReason::TakeProfit, 45_000, 1),
            make_trade(-5.0, -0.0005, ExitReason::HardStop, 30_000, 2),
        ],
    );

    let cmp = compare_configs(&result_a, &result_b);

    assert_eq!(cmp.config_a_id, "v1");
    assert_eq!(cmp.config_b_id, "v2");

    // v1 total_pnl = 30, v2 total_pnl = 85
    assert!((cmp.pnl_delta - 55.0).abs() < f64::EPSILON);
    assert_eq!(cmp.trades_delta, 1); // 3 - 2 = 1
}

#[test]
fn config_comparison_negative_delta() {
    let result_a = make_result(
        "good",
        vec![
            make_trade(100.0, 0.01, ExitReason::TrailingStop, 60_000, 0),
        ],
    );
    let result_b = make_result(
        "bad",
        vec![
            make_trade(-50.0, -0.005, ExitReason::HardStop, 30_000, 0),
        ],
    );

    let cmp = compare_configs(&result_a, &result_b);
    assert!(cmp.pnl_delta < 0.0, "worse config should show negative delta");
}

#[test]
fn comparison_summary_readable() {
    let result_a = make_result(
        "v1",
        vec![make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0)],
    );
    let result_b = make_result(
        "v2",
        vec![make_trade(80.0, 0.008, ExitReason::TrailingStop, 60_000, 0)],
    );
    let cmp = compare_configs(&result_a, &result_b);
    let text = comparison_summary(&cmp);
    assert!(text.contains("config comparison"));
    assert!(text.contains("v1"));
    assert!(text.contains("v2"));
    assert!(text.contains("P&L:"));
    assert!(text.contains("sharpe:"));
}

#[test]
fn comparison_serializes_to_json() {
    let result_a = make_result(
        "v1",
        vec![make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0)],
    );
    let result_b = make_result(
        "v2",
        vec![make_trade(80.0, 0.008, ExitReason::TrailingStop, 60_000, 0)],
    );
    let cmp = compare_configs(&result_a, &result_b);
    let json = serde_json::to_string(&cmp).unwrap();
    assert!(json.contains("pnl_delta"));
    assert!(json.contains("config_a_id"));
}
