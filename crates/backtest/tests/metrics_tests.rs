use chrono::{TimeZone, Utc};
use engine::TradeRecord;
use types::action::{ExitReason, TradeDirection};

use backtest::compute_metrics;

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
    let entry_price = 100.0;
    let exit_price = if pnl > 0.0 {
        entry_price + pnl / 100.0
    } else {
        entry_price + pnl / 100.0
    };

    TradeRecord {
        ticker: "SPY".to_string(),
        direction: TradeDirection::Long,
        entry_price,
        exit_price,
        size: 100.0,
        entry_time,
        exit_time,
        pnl,
        pnl_pct,
        hold_duration_ms: hold_ms,
        exit_reason,
        high_water_mark: entry_price + 2.0,
        low_water_mark: entry_price - 1.0,
    }
}

fn default_period() -> (chrono::DateTime<Utc>, chrono::DateTime<Utc>) {
    let start = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
    let end = Utc.timestamp_opt(1_700_000_000 + 10 * 86400, 0).unwrap(); // 10 days
    (start, end)
}

#[test]
fn empty_trades_returns_zero_metrics() {
    let (start, end) = default_period();
    let (metrics, equity_curve) = compute_metrics(&[], 10_000.0, start, end, &[]);
    assert_eq!(metrics.total_trades, 0);
    assert!((metrics.total_pnl - 0.0).abs() < f64::EPSILON);
    assert!((metrics.win_rate - 0.0).abs() < f64::EPSILON);
    assert!((metrics.max_drawdown - 0.0).abs() < f64::EPSILON);
    assert!(equity_curve.is_empty());
}

#[test]
fn total_pnl_sums_correctly() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        make_trade(100.0, 0.01, ExitReason::TakeProfit, 120_000, 2),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert!((metrics.total_pnl - 130.0).abs() < f64::EPSILON);
    assert!((metrics.total_pnl_pct - 0.013).abs() < 1e-9);
}

#[test]
fn win_rate_computed_correctly() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        make_trade(100.0, 0.01, ExitReason::TakeProfit, 120_000, 2),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert_eq!(metrics.winning_trades, 2);
    assert_eq!(metrics.losing_trades, 1);
    assert!((metrics.win_rate - 2.0 / 3.0).abs() < 1e-9);
}

#[test]
fn avg_win_and_loss_computed_correctly() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(60.0, 0.006, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        make_trade(40.0, 0.004, ExitReason::TakeProfit, 120_000, 2),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert!((metrics.avg_win - 50.0).abs() < f64::EPSILON);
    assert!((metrics.avg_loss - (-20.0)).abs() < f64::EPSILON);
}

#[test]
fn profit_factor_computed_correctly() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(100.0, 0.01, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-50.0, -0.005, ExitReason::HardStop, 30_000, 1),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert!((metrics.profit_factor - 2.0).abs() < 1e-9);
}

#[test]
fn profit_factor_all_winners_is_infinite() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(100.0, 0.01, ExitReason::TakeProfit, 120_000, 1),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert!((metrics.profit_factor - f64::MAX).abs() < f64::EPSILON);
}

#[test]
fn max_drawdown_tracked_correctly() {
    let (start, end) = default_period();
    // sequence: +100, -50, -30 → cumulative equity: 10100, 10050, 10020
    // peak at 10100, max DD = 10100 - 10020 = 80
    let trades = vec![
        make_trade(100.0, 0.01, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-50.0, -0.005, ExitReason::HardStop, 30_000, 1),
        make_trade(-30.0, -0.003, ExitReason::HardStop, 30_000, 2),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert!((metrics.max_drawdown - 80.0).abs() < f64::EPSILON);
}

#[test]
fn max_drawdown_pct_correct() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(100.0, 0.01, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-50.0, -0.005, ExitReason::HardStop, 30_000, 1),
        make_trade(-30.0, -0.003, ExitReason::HardStop, 30_000, 2),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    // peak = 10100, dd = 80, pct = 80/10100
    let expected_pct = 80.0 / 10100.0;
    assert!((metrics.max_drawdown_pct - expected_pct).abs() < 1e-9);
}

#[test]
fn equity_curve_tracks_cumulative() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        make_trade(30.0, 0.003, ExitReason::TakeProfit, 120_000, 2),
    ];
    let (_, equity_curve) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert_eq!(equity_curve.len(), 3);
    assert!((equity_curve[0].equity - 10_050.0).abs() < f64::EPSILON);
    assert!((equity_curve[1].equity - 10_030.0).abs() < f64::EPSILON);
    assert!((equity_curve[2].equity - 10_060.0).abs() < f64::EPSILON);
}

#[test]
fn avg_hold_duration_computed() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 120_000, 1),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert_eq!(metrics.avg_hold_duration_ms, 90_000);
}

#[test]
fn trades_per_day_computed() {
    let (start, end) = default_period(); // 10 day period
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        make_trade(30.0, 0.003, ExitReason::TakeProfit, 120_000, 2),
        make_trade(10.0, 0.001, ExitReason::SessionClose, 60_000, 3),
        make_trade(-15.0, -0.0015, ExitReason::MaxHoldTimeout, 60_000, 4),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert!((metrics.trades_per_day - 0.5).abs() < 1e-9); // 5 trades / 10 days
}

#[test]
fn sharpe_ratio_positive_for_consistent_wins() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(40.0, 0.004, ExitReason::TrailingStop, 60_000, 1),
        make_trade(60.0, 0.006, ExitReason::TrailingStop, 60_000, 2),
        make_trade(45.0, 0.0045, ExitReason::TrailingStop, 60_000, 3),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    assert!(metrics.sharpe_ratio > 0.0, "consistent wins should produce positive sharpe");
}

#[test]
fn trade_sharpe_zero_for_identical_returns() {
    let (start, end) = default_period();
    // all same per-trade return → std = 0 → trade_sharpe = 0
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 1),
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 2),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    // per-trade sharpe should be 0 (identical returns)
    assert!((metrics.trade_sharpe_ratio - 0.0).abs() < f64::EPSILON);
    // daily sharpe may be non-zero since daily returns differ slightly
    // (equity changes between days, so return = pnl/equity changes)
}

#[test]
fn exit_reason_breakdown() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-20.0, -0.002, ExitReason::HardStop, 30_000, 1),
        make_trade(100.0, 0.01, ExitReason::TrailingStop, 120_000, 2),
    ];
    let (metrics, _) = compute_metrics(&trades, 10_000.0, start, end, &[]);

    let trailing = metrics.by_exit_reason.get(&ExitReason::TrailingStop).unwrap();
    assert_eq!(trailing.count, 2);
    assert!((trailing.total_pnl - 150.0).abs() < f64::EPSILON);
    assert!((trailing.avg_pnl - 75.0).abs() < f64::EPSILON);
    assert!((trailing.win_rate - 1.0).abs() < f64::EPSILON);

    let hard = metrics.by_exit_reason.get(&ExitReason::HardStop).unwrap();
    assert_eq!(hard.count, 1);
    assert!((hard.total_pnl - (-20.0)).abs() < f64::EPSILON);
    assert!((hard.win_rate - 0.0).abs() < f64::EPSILON);
}

#[test]
fn backtest_result_serializes_to_json() {
    let (start, end) = default_period();
    let trades = vec![
        make_trade(50.0, 0.005, ExitReason::TrailingStop, 60_000, 0),
        make_trade(-10.0, -0.001, ExitReason::HardStop, 30_000, 1),
    ];
    let (metrics, equity_curve) = compute_metrics(&trades, 10_000.0, start, end, &[]);
    let result = backtest::BacktestResult {
        config_id: "test_v1".to_string(),
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
    };
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert!(json.contains("\"config_id\""));
    assert!(json.contains("\"total_pnl\""));
    assert!(json.contains("\"equity\""));
    // verify it roundtrips
    let deserialized: backtest::BacktestResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.config_id, "test_v1");
    assert_eq!(deserialized.metrics.total_trades, 2);
}
