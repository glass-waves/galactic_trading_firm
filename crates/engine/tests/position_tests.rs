use chrono::{Duration, Utc};
use types::action::{ExitReason, TradeDirection};

use engine::PositionManager;

#[test]
fn no_position_initially() {
    let pm = PositionManager::new();
    assert!(!pm.has_position());
    assert!(pm.current_position().is_none());
}

#[test]
fn open_position_sets_fields() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("SPY".into(), TradeDirection::Long, 100.0, 50.0, now).unwrap();

    let pos = pm.current_position().unwrap();
    assert_eq!(pos.ticker, "SPY");
    assert!(matches!(pos.direction, TradeDirection::Long));
    assert!((pos.entry_price - 100.0).abs() < f64::EPSILON);
    assert!((pos.current_price - 100.0).abs() < f64::EPSILON);
    assert!((pos.size - 50.0).abs() < f64::EPSILON);
    assert!((pos.unrealized_pnl - 0.0).abs() < f64::EPSILON);
    assert!((pos.high_water_mark - 100.0).abs() < f64::EPSILON);
    assert!((pos.low_water_mark - 100.0).abs() < f64::EPSILON);
}

#[test]
fn update_on_tick_updates_price_and_pnl() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("SPY".into(), TradeDirection::Long, 100.0, 50.0, now).unwrap();

    let later = now + Duration::milliseconds(60_000);
    pm.update_on_tick(103.0, later);

    let pos = pm.current_position().unwrap();
    assert!((pos.current_price - 103.0).abs() < f64::EPSILON);
    assert!((pos.unrealized_pnl - 150.0).abs() < f64::EPSILON); // (103-100)*50
    assert!((pos.unrealized_pnl_pct - 0.03).abs() < 1e-10);
}

#[test]
fn high_water_mark_tracks_peak() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("SPY".into(), TradeDirection::Long, 100.0, 50.0, now).unwrap();

    pm.update_on_tick(105.0, now + Duration::milliseconds(1000));
    pm.update_on_tick(103.0, now + Duration::milliseconds(2000));
    pm.update_on_tick(107.0, now + Duration::milliseconds(3000));
    pm.update_on_tick(104.0, now + Duration::milliseconds(4000));

    let pos = pm.current_position().unwrap();
    assert!((pos.high_water_mark - 107.0).abs() < f64::EPSILON);
}

#[test]
fn low_water_mark_tracks_trough() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("SPY".into(), TradeDirection::Long, 100.0, 50.0, now).unwrap();

    pm.update_on_tick(98.0, now + Duration::milliseconds(1000));
    pm.update_on_tick(102.0, now + Duration::milliseconds(2000));
    pm.update_on_tick(97.0, now + Duration::milliseconds(3000));
    pm.update_on_tick(99.0, now + Duration::milliseconds(4000));

    let pos = pm.current_position().unwrap();
    assert!((pos.low_water_mark - 97.0).abs() < f64::EPSILON);
}

#[test]
fn close_position_returns_trade_record() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("QQQ".into(), TradeDirection::Long, 100.0, 100.0, now).unwrap();

    let exit_time = now + Duration::milliseconds(120_000);
    pm.update_on_tick(105.0, exit_time);

    let record = pm.close_position(105.0, exit_time, ExitReason::TrailingStop).unwrap();
    assert_eq!(record.ticker, "QQQ");
    assert!((record.entry_price - 100.0).abs() < f64::EPSILON);
    assert!((record.exit_price - 105.0).abs() < f64::EPSILON);
    assert!((record.pnl - 500.0).abs() < f64::EPSILON); // (105-100)*100
    assert!((record.pnl_pct - 0.05).abs() < 1e-10);
    assert_eq!(record.exit_reason, ExitReason::TrailingStop);
    assert_eq!(record.hold_duration_ms, 120_000);
    assert!(!pm.has_position());
}

#[test]
fn hold_duration_increases() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("SPY".into(), TradeDirection::Long, 100.0, 50.0, now).unwrap();

    pm.update_on_tick(101.0, now + Duration::milliseconds(30_000));
    assert_eq!(pm.current_position().unwrap().hold_duration_ms, 30_000);

    pm.update_on_tick(102.0, now + Duration::milliseconds(90_000));
    assert_eq!(pm.current_position().unwrap().hold_duration_ms, 90_000);
}

#[test]
fn short_position_pnl_inverted() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("SPY".into(), TradeDirection::Short, 100.0, 50.0, now).unwrap();

    pm.update_on_tick(95.0, now + Duration::milliseconds(1000));
    let pos = pm.current_position().unwrap();
    // short: profit when price goes down
    assert!((pos.unrealized_pnl - 250.0).abs() < f64::EPSILON); // (100-95)*50
    assert!((pos.unrealized_pnl_pct - 0.05).abs() < 1e-10);

    let exit_time = now + Duration::milliseconds(2000);
    let record = pm.close_position(95.0, exit_time, ExitReason::TakeProfit).unwrap();
    assert!((record.pnl - 250.0).abs() < f64::EPSILON);
}

#[test]
fn cannot_open_when_position_exists() {
    let mut pm = PositionManager::new();
    let now = Utc::now();
    pm.open_position("SPY".into(), TradeDirection::Long, 100.0, 50.0, now).unwrap();
    let result = pm.open_position("QQQ".into(), TradeDirection::Short, 200.0, 30.0, now);
    assert!(result.is_err());
}
