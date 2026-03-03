use types::indicator::Indicator;
use types::market::{PositionContext, Timescale};
use types::test_fixtures::{make_indicator_config, make_market_state, make_market_state_ohlcv};

use indicators::custom::ofi::OfiIndicator;
use indicators::custom::position_context::{
    position_direction_factory, unrealized_pnl_factory, hold_duration_factory,
    session_remaining_factory,
};
use indicators::default_indicator_registry;

fn bullish_ohlcv(n: usize) -> Vec<(f64, f64, f64, f64, f64)> {
    // close near high → positive CLV
    (0..n)
        .map(|i| {
            let base = 100.0 + i as f64;
            (base, base + 2.0, base - 1.0, base + 1.8, 100_000.0)
        })
        .collect()
}

fn bearish_ohlcv(n: usize) -> Vec<(f64, f64, f64, f64, f64)> {
    // close near low → negative CLV
    (0..n)
        .map(|i| {
            let base = 100.0 - i as f64 * 0.5;
            (base, base + 1.0, base - 2.0, base - 1.8, 100_000.0)
        })
        .collect()
}

#[test]
fn ofi_insufficient_data_returns_none() {
    let ofi = OfiIndicator::new(20, Timescale::FiveMinute, "ofi_test".to_string());
    // only 10 candles, needs 21
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &bullish_ohlcv(10));
    assert!(ofi.compute(&market).is_none());
}

#[test]
fn ofi_score_in_valid_range() {
    let ofi = OfiIndicator::new(20, Timescale::FiveMinute, "ofi_test".to_string());
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &bullish_ohlcv(30));
    let output = ofi.compute(&market).expect("should produce output");
    assert!(output.score >= -1.0 && output.score <= 1.0, "score {} out of range", output.score);
}

#[test]
fn ofi_bullish_candles_positive() {
    let ofi = OfiIndicator::new(20, Timescale::FiveMinute, "ofi_test".to_string());
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &bullish_ohlcv(30));
    let output = ofi.compute(&market).expect("should produce output");
    assert!(output.score > 0.0, "bullish candles should produce positive score, got {}", output.score);
}

#[test]
fn ofi_bearish_candles_negative() {
    let ofi = OfiIndicator::new(20, Timescale::FiveMinute, "ofi_test".to_string());
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &bearish_ohlcv(30));
    let output = ofi.compute(&market).expect("should produce output");
    assert!(output.score < 0.0, "bearish candles should produce negative score, got {}", output.score);
}

#[test]
fn ofi_doji_handles_zero_range() {
    // high == low → CLV should be 0, no panic
    let mut data = bullish_ohlcv(25);
    // make last candle a doji (high == low)
    let last = data.last_mut().unwrap();
    *last = (100.0, 100.0, 100.0, 100.0, 100_000.0);
    let ofi = OfiIndicator::new(20, Timescale::FiveMinute, "ofi_test".to_string());
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let output = ofi.compute(&market).expect("should produce output for doji");
    assert!((output.score - 0.0).abs() < f64::EPSILON, "doji should produce 0 score, got {}", output.score);
}

#[test]
fn ofi_extreme_volume_clamped() {
    // 10x average volume → score should be clamped at ±1.0
    let mut data = bullish_ohlcv(25);
    let last = data.last_mut().unwrap();
    // close near high with 10x volume
    *last = (last.0, last.1, last.2, last.3, 1_000_000.0);
    let ofi = OfiIndicator::new(20, Timescale::FiveMinute, "ofi_test".to_string());
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let output = ofi.compute(&market).expect("should produce output");
    assert!(output.score <= 1.0 && output.score >= -1.0, "extreme volume should be clamped, got {}", output.score);
}

#[test]
fn ofi_factory_creates_from_config() {
    let registry = default_indicator_registry();
    let config = make_indicator_config(
        "ofi",
        "ofi_20_5min",
        Timescale::FiveMinute,
        1.0,
        vec![("avg_period", serde_json::json!(20))],
    );
    let indicator = (registry.factories.get("ofi").unwrap())(&config);
    assert_eq!(indicator.name(), "ofi");
    assert_eq!(indicator.timescale(), Timescale::FiveMinute);
}

#[test]
fn ofi_name_and_timescale() {
    let ofi = OfiIndicator::new(20, Timescale::OneMinute, "ofi_test".to_string());
    assert_eq!(ofi.name(), "ofi");
    assert_eq!(ofi.timescale(), Timescale::OneMinute);
}

#[test]
fn ofi_metadata_keys() {
    let ofi = OfiIndicator::new(20, Timescale::FiveMinute, "ofi_test".to_string());
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &bullish_ohlcv(30));
    let output = ofi.compute(&market).expect("should produce output");
    assert!(output.metadata.contains_key("raw_ofi"), "missing raw_ofi");
    assert!(output.metadata.contains_key("clv"), "missing clv");
    assert!(output.metadata.contains_key("avg_volume"), "missing avg_volume");
}

// ── Position Context Meta-Indicators ──

fn make_market_with_position(direction: f64, pnl_pct: f64, hold_ms: i64) -> types::market::MarketState {
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    ms.position_context = Some(PositionContext {
        direction,
        unrealized_pnl_pct: pnl_pct,
        hold_duration_ms: hold_ms,
        max_hold_ms: 2_700_000,
    });
    ms
}

fn make_market_with_session(progress: f64) -> types::market::MarketState {
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    ms.session_progress = Some(progress);
    ms
}

#[test]
fn position_direction_flat() {
    let config = make_indicator_config("position_direction", "pd", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = position_direction_factory(&config);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]); // no position
    assert!(indicator.compute(&ms).is_none());
}

#[test]
fn position_direction_long() {
    let config = make_indicator_config("position_direction", "pd", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = position_direction_factory(&config);
    let ms = make_market_with_position(1.0, 0.01, 60_000);
    let output = indicator.compute(&ms).expect("should produce output");
    assert!((output.score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn position_direction_short() {
    let config = make_indicator_config("position_direction", "pd", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = position_direction_factory(&config);
    let ms = make_market_with_position(-1.0, -0.005, 60_000);
    let output = indicator.compute(&ms).expect("should produce output");
    assert!((output.score - (-1.0)).abs() < f64::EPSILON);
}

#[test]
fn unrealized_pnl_positive() {
    let config = make_indicator_config("unrealized_pnl", "upnl", Timescale::FiveMinute, 1.0, vec![("scale", serde_json::json!(100.0))]);
    let indicator = unrealized_pnl_factory(&config);
    let ms = make_market_with_position(1.0, 0.015, 60_000); // +1.5%
    let output = indicator.compute(&ms).expect("should produce output");
    assert!(output.score > 0.0, "positive pnl should give positive score, got {}", output.score);
    assert!(output.score <= 1.0);
}

#[test]
fn holding_duration_fresh() {
    let config = make_indicator_config("hold_duration", "hd", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = hold_duration_factory(&config);
    let ms = make_market_with_position(1.0, 0.001, 1_000); // just entered
    let output = indicator.compute(&ms).expect("should produce output");
    assert!(output.score < 0.01, "fresh position should be near 0, got {}", output.score);
}

#[test]
fn holding_duration_near_timeout() {
    let config = make_indicator_config("hold_duration", "hd", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = hold_duration_factory(&config);
    let ms = make_market_with_position(1.0, 0.001, 2_600_000); // near max 2,700,000
    let output = indicator.compute(&ms).expect("should produce output");
    assert!(output.score > 0.9, "near-timeout should be near 1.0, got {}", output.score);
}

#[test]
fn session_remaining_open() {
    let config = make_indicator_config("session_remaining", "sr", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = session_remaining_factory(&config);
    let ms = make_market_with_session(0.1); // early in session
    let output = indicator.compute(&ms).expect("should produce output");
    assert!(output.score > 0.8, "early session should give high score, got {}", output.score);
}

#[test]
fn session_remaining_close() {
    let config = make_indicator_config("session_remaining", "sr", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = session_remaining_factory(&config);
    let ms = make_market_with_session(0.95); // near close
    let output = indicator.compute(&ms).expect("should produce output");
    assert!(output.score < 0.1, "near close should give low score, got {}", output.score);
}

#[test]
fn meta_indicator_no_context_returns_none() {
    let config = make_indicator_config("position_direction", "pd", Timescale::FiveMinute, 1.0, vec![]);
    let indicator = position_direction_factory(&config);
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]); // no context
    assert!(indicator.compute(&ms).is_none(), "no position context → None");

    let config2 = make_indicator_config("session_remaining", "sr", Timescale::FiveMinute, 1.0, vec![]);
    let indicator2 = session_remaining_factory(&config2);
    assert!(indicator2.compute(&ms).is_none(), "no session progress → None");
}
