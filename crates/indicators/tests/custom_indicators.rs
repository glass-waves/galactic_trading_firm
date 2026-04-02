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

// ── RvolIndicator ──

use indicators::custom::rvol::RvolIndicator;

#[test]
fn rvol_high_volume_positive_score() {
    let ind = RvolIndicator::new(5, 1.5, 0.5, Timescale::FiveMinute, "rvol_5".into());
    // 5 candles with 100k volume, then 1 candle with 200k (2x avg → RVOL=2.0 > 1.5)
    let mut ohlcv: Vec<(f64, f64, f64, f64, f64)> = (0..5)
        .map(|i| (100.0 + i as f64, 102.0, 98.0, 101.0, 100_000.0))
        .collect();
    ohlcv.push((105.0, 107.0, 103.0, 106.0, 200_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &ohlcv);
    let out = ind.compute(&ms).expect("should compute");
    assert!(out.score > 0.0, "high rvol should give positive score, got {}", out.score);
}

#[test]
fn rvol_low_volume_negative_score() {
    let ind = RvolIndicator::new(5, 1.5, 0.5, Timescale::FiveMinute, "rvol_5".into());
    // 5 candles with 100k volume, then 1 candle with 30k (0.3x avg → RVOL=0.3 < 0.5)
    let mut ohlcv: Vec<(f64, f64, f64, f64, f64)> = (0..5)
        .map(|i| (100.0 + i as f64, 102.0, 98.0, 101.0, 100_000.0))
        .collect();
    ohlcv.push((105.0, 107.0, 103.0, 106.0, 30_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &ohlcv);
    let out = ind.compute(&ms).expect("should compute");
    assert!(out.score < 0.0, "low rvol should give negative score, got {}", out.score);
}

#[test]
fn rvol_insufficient_data_returns_none() {
    let ind = RvolIndicator::new(20, 1.5, 0.5, Timescale::FiveMinute, "rvol_20".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 5]);
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn rvol_normal_volume_zero_score() {
    let ind = RvolIndicator::new(5, 1.5, 0.5, Timescale::FiveMinute, "rvol_5".into());
    let ohlcv: Vec<(f64, f64, f64, f64, f64)> = (0..6)
        .map(|i| (100.0 + i as f64, 102.0, 98.0, 101.0, 100_000.0))
        .collect();
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &ohlcv);
    let out = ind.compute(&ms).expect("should compute");
    assert!((out.score - 0.0).abs() < 0.01, "normal volume should give ~0 score, got {}", out.score);
}

// ── MarketBreadthIndicator ──

use indicators::custom::market_breadth::MarketBreadthIndicator;

#[test]
fn market_breadth_outperformance_positive() {
    let ind = MarketBreadthIndicator::new(5, 0.02, Timescale::FiveMinute, "mb_5".into());
    // stock up 5% over 5 candles, index up 1%
    let ohlcv: Vec<(f64, f64, f64, f64, f64)> = (0..6)
        .map(|i| {
            let c = 100.0 + i as f64;
            (c, c + 1.0, c - 1.0, c, 100_000.0)
        })
        .collect();
    let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &ohlcv);
    ms.index_return = Some(0.01); // index up 1%
    let out = ind.compute(&ms).expect("should compute");
    assert!(out.score > 0.0, "outperformance should give positive score, got {}", out.score);
}

#[test]
fn market_breadth_no_index_returns_none() {
    let ind = MarketBreadthIndicator::new(5, 0.02, Timescale::FiveMinute, "mb_5".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 10]);
    assert!(ind.compute(&ms).is_none(), "no index_return → None");
}

// ── CrossCorrelationIndicator ──

use indicators::custom::cross_correlation::CrossCorrelationIndicator;

#[test]
fn cross_corr_high_gives_negative() {
    let ind = CrossCorrelationIndicator::new(0.8, 0.3, Timescale::FiveMinute, "cc_5".into());
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    ms.cross_ticker_correlation = Some(0.9);
    let out = ind.compute(&ms).expect("should compute");
    assert_eq!(out.score, -1.0);
}

#[test]
fn cross_corr_low_gives_positive() {
    let ind = CrossCorrelationIndicator::new(0.8, 0.3, Timescale::FiveMinute, "cc_5".into());
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    ms.cross_ticker_correlation = Some(0.2);
    let out = ind.compute(&ms).expect("should compute");
    assert_eq!(out.score, 1.0);
}

#[test]
fn cross_corr_mid_interpolates() {
    let ind = CrossCorrelationIndicator::new(0.8, 0.3, Timescale::FiveMinute, "cc_5".into());
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    ms.cross_ticker_correlation = Some(0.55); // midpoint
    let out = ind.compute(&ms).expect("should compute");
    assert!((out.score - 0.0).abs() < 0.01, "midpoint should give ~0, got {}", out.score);
}

#[test]
fn cross_corr_no_data_returns_none() {
    let ind = CrossCorrelationIndicator::new(0.8, 0.3, Timescale::FiveMinute, "cc_5".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    assert!(ind.compute(&ms).is_none());
}

// ── CandlePattern Indicator ──

use indicators::custom::candle_pattern::CandlePattern;

fn make_candle_pattern(mean_reversion: bool, use_confluence: bool) -> CandlePattern {
    CandlePattern::new(
        Timescale::FiveMinute,
        "cp_test".to_string(),
        0.5,  // engulfing_min_body_ratio
        0.70, // engulfing_base_score
        0.60, // hammer_base_score
        2.0,  // hammer_wick_ratio
        0.30, // hammer_max_body_pct
        0.60, // hammer_min_wick_pct
        0.65, // evening_star_base_score
        0.50, // evening_star_min_body_pct
        0.20, // evening_star_max_doji_pct
        0.75, // confirmed_engulfing_base_score
        mean_reversion,
        20,   // volume_lookback
        2.0,  // high_volume_threshold
        0.5,  // low_volume_threshold
        5,    // ema_fast
        20,   // ema_slow
        use_confluence,
        false, // affirmative_only (false for tests to verify both positive and negative scores)
        0.0,   // min_confluence_product (0.0 = no gate, for test coverage)
        1,     // decay_candles (1 = no lookback, matches original behavior for tests)
        1.0,   // decay_factor (1.0 = no decay)
    )
}

/// build OHLCV data: N-2 neutral candles + 2 final candles for pattern testing.
fn engulfing_ohlcv(
    n: usize,
    prev: (f64, f64, f64, f64, f64),
    curr: (f64, f64, f64, f64, f64),
) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..n.saturating_sub(2))
        .map(|i| {
            let base = 100.0 + i as f64 * 0.1;
            (base, base + 1.0, base - 1.0, base + 0.1, 100_000.0)
        })
        .collect();
    data.push(prev);
    data.push(curr);
    data
}

// ── engulfing detection ──

#[test]
fn candle_bullish_engulfing_detected() {
    let ind = make_candle_pattern(false, false);
    // prev: bearish (open=102, close=100), curr: bullish (open=99, close=103) — wraps prev body
    let data = engulfing_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0), // bearish prev
        (99.0, 104.0, 98.0, 103.0, 100_000.0),   // bullish curr, body 99-103 wraps 100-102
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score > 0.0, "bullish engulfing should be positive, got {}", out.score);
}

#[test]
fn candle_bearish_engulfing_detected() {
    let ind = make_candle_pattern(false, false);
    // prev: bullish (open=100, close=102), curr: bearish (open=103, close=99) — wraps prev body
    let data = engulfing_ohlcv(
        25,
        (100.0, 102.5, 99.5, 102.0, 100_000.0), // bullish prev
        (103.0, 104.0, 98.0, 99.0, 100_000.0),   // bearish curr, body 99-103 wraps 100-102
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score < 0.0, "bearish engulfing should be negative, got {}", out.score);
}

#[test]
fn candle_body_too_small_not_engulfing() {
    let ind = make_candle_pattern(false, false);
    // curr body_ratio < 0.5 (tiny body in large range)
    let data = engulfing_ohlcv(
        25,
        (101.0, 102.0, 99.0, 100.0, 100_000.0),  // bearish prev
        (99.9, 110.0, 90.0, 100.1, 100_000.0),    // bullish but body=0.2, range=20, ratio=0.01
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!((out.score - 0.0).abs() < f64::EPSILON, "small body should not be engulfing, got {}", out.score);
}

#[test]
fn candle_same_direction_not_engulfing() {
    let ind = make_candle_pattern(false, false);
    // both bullish — not an engulfing pattern
    let data = engulfing_ohlcv(
        25,
        (100.0, 103.0, 99.0, 102.0, 100_000.0),  // bullish
        (101.0, 105.0, 100.0, 104.0, 100_000.0),  // also bullish
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!((out.score - 0.0).abs() < f64::EPSILON, "same direction should not match, got {}", out.score);
}

#[test]
fn candle_no_contain_not_engulfing() {
    let ind = make_candle_pattern(false, false);
    // curr body does NOT fully contain prev body (prev body top=102, curr body top=101.5)
    let data = engulfing_ohlcv(
        25,
        (102.0, 103.0, 99.0, 100.0, 100_000.0),  // bearish: body 100-102
        (99.5, 103.0, 98.0, 101.5, 100_000.0),    // bullish: body 99.5-101.5 — doesn't reach 102
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!((out.score - 0.0).abs() < f64::EPSILON, "incomplete wrap should not match, got {}", out.score);
}

#[test]
fn candle_zero_range_no_panic() {
    let ind = make_candle_pattern(false, false);
    // zero range candle (high == low)
    let data = engulfing_ohlcv(
        25,
        (100.0, 102.0, 99.0, 101.0, 100_000.0),
        (100.0, 100.0, 100.0, 100.0, 100_000.0), // zero range
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output without panic");
    assert!((out.score - 0.0).abs() < f64::EPSILON);
}

// ── confluence scoring ──

#[test]
fn candle_high_volume_increases_score() {
    let ind = make_candle_pattern(false, true);
    // bullish engulfing with 2.5x average volume
    let mut data = engulfing_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),
        (99.0, 104.0, 98.0, 103.0, 250_000.0), // 2.5x volume
    );
    // ensure preceding candles have 100k volume (already default from engulfing_ohlcv)
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out_high = ind.compute(&ms).expect("should produce output");

    // same pattern with normal volume
    data.last_mut().unwrap().4 = 100_000.0;
    let ms_normal = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out_normal = ind.compute(&ms_normal).expect("should produce output");

    assert!(
        out_high.score.abs() > out_normal.score.abs(),
        "high volume should increase score: {} vs {}",
        out_high.score.abs(), out_normal.score.abs()
    );
}

#[test]
fn candle_low_volume_decreases_score() {
    let ind = make_candle_pattern(false, true);
    let mut data = engulfing_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),
        (99.0, 104.0, 98.0, 103.0, 30_000.0), // 0.3x volume
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out_low = ind.compute(&ms).expect("should produce output");

    data.last_mut().unwrap().4 = 100_000.0;
    let ms_normal = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out_normal = ind.compute(&ms_normal).expect("should produce output");

    assert!(
        out_low.score.abs() < out_normal.score.abs(),
        "low volume should decrease score: {} vs {}",
        out_low.score.abs(), out_normal.score.abs()
    );
}

#[test]
fn candle_below_vwap_reversal_boosts() {
    let ind = make_candle_pattern(false, true);
    // bullish engulfing
    let data = engulfing_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),
        (99.0, 104.0, 98.0, 103.0, 100_000.0),
    );
    let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    ms.session_vwap = 110.0; // price well below VWAP → location boost
    let out_below = ind.compute(&ms).expect("should produce output");

    ms.session_vwap = 95.0; // price above VWAP → location discount
    let out_above = ind.compute(&ms).expect("should produce output");

    assert!(
        out_below.score.abs() > out_above.score.abs(),
        "below VWAP should boost: {} vs {}",
        out_below.score.abs(), out_above.score.abs()
    );
}

#[test]
fn candle_above_vwap_reversal_discounts() {
    let ind = make_candle_pattern(false, true);
    // bullish engulfing with close above VWAP
    let data = engulfing_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),
        (99.0, 104.0, 98.0, 103.0, 100_000.0),
    );
    let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    ms.session_vwap = 95.0; // above VWAP
    let out = ind.compute(&ms).expect("should produce output");
    // check metadata for location_mult
    let loc_mult = out.metadata.get("location_mult").copied().unwrap_or(1.0);
    assert!((loc_mult - 0.8).abs() < f64::EPSILON, "above VWAP bullish should get 0.8 mult, got {}", loc_mult);
}

#[test]
fn candle_with_trend_boosts() {
    let ind = make_candle_pattern(false, true);
    // bullish engulfing in uptrend (EMA5 > EMA20 when prices trending up)
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..23)
        .map(|i| {
            let base = 90.0 + i as f64; // clear uptrend
            (base, base + 1.0, base - 1.0, base + 0.5, 100_000.0)
        })
        .collect();
    // add engulfing at the end
    data.push((115.0, 116.0, 111.0, 112.0, 100_000.0)); // bearish prev
    data.push((111.0, 118.0, 110.0, 116.0, 100_000.0));  // bullish curr wraps prev
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    let trend_mult = out.metadata.get("trend_mult").copied().unwrap_or(1.0);
    assert!((trend_mult - 1.1).abs() < f64::EPSILON, "with-trend bullish should get 1.1, got {}", trend_mult);
}

#[test]
fn candle_counter_trend_discounts() {
    let ind = make_candle_pattern(false, true);
    // bullish engulfing in downtrend (EMA5 < EMA20 when prices trending down)
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..23)
        .map(|i| {
            let base = 120.0 - i as f64; // clear downtrend
            (base, base + 1.0, base - 1.0, base - 0.5, 100_000.0)
        })
        .collect();
    data.push((99.0, 100.0, 96.0, 97.0, 100_000.0));  // bearish prev
    data.push((96.0, 101.0, 95.0, 100.0, 100_000.0));  // bullish curr wraps prev
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    let trend_mult = out.metadata.get("trend_mult").copied().unwrap_or(1.0);
    assert!((trend_mult - 0.8).abs() < f64::EPSILON, "counter-trend bullish should get 0.8, got {}", trend_mult);
}

// ── mean reversion mode ──

#[test]
fn candle_mean_reversion_flips_bearish_to_positive() {
    let ind = make_candle_pattern(true, false);
    // bearish engulfing → should score POSITIVE in mean reversion mode
    let data = engulfing_ohlcv(
        25,
        (100.0, 102.5, 99.5, 102.0, 100_000.0), // bullish prev
        (103.0, 104.0, 98.0, 99.0, 100_000.0),   // bearish curr wraps
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score > 0.0, "mean reversion: bearish engulfing should be positive, got {}", out.score);
}

#[test]
fn candle_mean_reversion_flips_bullish_to_negative() {
    let ind = make_candle_pattern(true, false);
    // bullish engulfing → should score NEGATIVE in mean reversion mode
    let data = engulfing_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0), // bearish prev
        (99.0, 104.0, 98.0, 103.0, 100_000.0),   // bullish curr wraps
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score < 0.0, "mean reversion: bullish engulfing should be negative, got {}", out.score);
}

#[test]
fn candle_traditional_mode_bearish_is_negative() {
    let ind = make_candle_pattern(false, false);
    let data = engulfing_ohlcv(
        25,
        (100.0, 102.5, 99.5, 102.0, 100_000.0),
        (103.0, 104.0, 98.0, 99.0, 100_000.0),
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score < 0.0, "traditional: bearish engulfing should be negative, got {}", out.score);
}

// ── integration ──

#[test]
fn candle_no_pattern_returns_zero() {
    let ind = make_candle_pattern(false, false);
    // two neutral candles, no engulfing
    let data = engulfing_ohlcv(
        25,
        (100.0, 102.0, 99.0, 101.0, 100_000.0), // bullish
        (101.0, 103.0, 100.0, 102.0, 100_000.0), // also bullish → no engulfing
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!((out.score - 0.0).abs() < f64::EPSILON, "no pattern should give 0.0, got {}", out.score);
}

#[test]
fn candle_insufficient_data_returns_none() {
    let ind = make_candle_pattern(false, false);
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &[(100.0, 102.0, 99.0, 101.0, 100_000.0)]);
    assert!(ind.compute(&ms).is_none(), "1 candle should return None");
}

#[test]
fn candle_confluence_multipliers_stack() {
    let ind = make_candle_pattern(false, true);
    // bullish engulfing + high volume + below VWAP + uptrend = all multipliers boost
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..23)
        .map(|i| {
            let base = 90.0 + i as f64;
            (base, base + 1.0, base - 1.0, base + 0.5, 100_000.0)
        })
        .collect();
    data.push((115.0, 116.0, 111.0, 112.0, 100_000.0)); // bearish prev
    data.push((111.0, 118.0, 110.0, 116.0, 250_000.0));  // bullish curr, high volume
    let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    ms.session_vwap = 120.0; // price below VWAP

    let out = ind.compute(&ms).expect("should produce output");
    // base 0.70 * vol 1.3 * loc 1.2 * trend 1.1 = 1.20
    // clamped to 1.0
    assert!((out.score - 1.0).abs() < f64::EPSILON, "stacked multipliers should clamp to 1.0, got {}", out.score);
}

#[test]
fn candle_score_clamped_to_range() {
    let ind = make_candle_pattern(false, true);
    // set up maximal multipliers
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..23)
        .map(|i| {
            let base = 90.0 + i as f64;
            (base, base + 1.0, base - 1.0, base + 0.5, 100_000.0)
        })
        .collect();
    data.push((115.0, 116.0, 111.0, 112.0, 100_000.0));
    data.push((111.0, 118.0, 110.0, 116.0, 500_000.0)); // 5x volume
    let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    ms.session_vwap = 200.0;

    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score >= -1.0 && out.score <= 1.0, "score must be in [-1, 1], got {}", out.score);
}

#[test]
fn candle_factory_creates_from_config() {
    let registry = default_indicator_registry();
    let config = make_indicator_config(
        "candle_pattern",
        "cp_5min",
        Timescale::FiveMinute,
        0.05,
        vec![
            ("mean_reversion_mode", serde_json::json!(true)),
            ("use_confluence", serde_json::json!(true)),
        ],
    );
    let indicator = (registry.factories.get("candle_pattern").unwrap())(&config);
    assert_eq!(indicator.name(), "candle_pattern");
    assert_eq!(indicator.timescale(), Timescale::FiveMinute);
}

#[test]
fn candle_metadata_contains_pattern_info() {
    let ind = make_candle_pattern(false, true);
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..23)
        .map(|i| {
            let base = 100.0 + i as f64 * 0.1;
            (base, base + 1.0, base - 1.0, base + 0.1, 100_000.0)
        })
        .collect();
    data.push((102.0, 103.0, 99.5, 100.0, 100_000.0));
    data.push((99.0, 104.0, 98.0, 103.0, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.metadata.contains_key("pattern_name"), "missing pattern_name");
    assert!(out.metadata.contains_key("body_ratio"), "missing body_ratio");
    assert!(out.metadata.contains_key("volume_ratio"), "missing volume_ratio");
    assert!(out.metadata.contains_key("volume_mult"), "missing volume_mult");
    assert!(out.metadata.contains_key("vwap_distance"), "missing vwap_distance");
    assert!(out.metadata.contains_key("location_mult"), "missing location_mult");
    assert!(out.metadata.contains_key("trend_mult"), "missing trend_mult");
}

// ── hammer / shooting star detection ──

/// build OHLCV data: N-1 neutral candles + 1 final candle for 1-candle pattern testing.
fn single_candle_ohlcv(
    n: usize,
    candle: (f64, f64, f64, f64, f64),
) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..n.saturating_sub(1))
        .map(|i| {
            let base = 100.0 + i as f64 * 0.1;
            (base, base + 1.0, base - 1.0, base + 0.1, 100_000.0)
        })
        .collect();
    data.push(candle);
    data
}

/// build OHLCV data: N-3 neutral candles + 3 final candles for 3-candle pattern testing.
fn three_candle_ohlcv(
    n: usize,
    c1: (f64, f64, f64, f64, f64),
    c2: (f64, f64, f64, f64, f64),
    c3: (f64, f64, f64, f64, f64),
) -> Vec<(f64, f64, f64, f64, f64)> {
    let mut data: Vec<(f64, f64, f64, f64, f64)> = (0..n.saturating_sub(3))
        .map(|i| {
            let base = 100.0 + i as f64 * 0.1;
            (base, base + 1.0, base - 1.0, base + 0.1, 100_000.0)
        })
        .collect();
    data.push(c1);
    data.push(c2);
    data.push(c3);
    data
}

#[test]
fn candle_hammer_detected() {
    let ind = make_candle_pattern(false, false);
    // hammer: small body at top, long lower wick
    // open=100, high=101, low=94, close=100.5 → body=0.5, range=7, lower_wick=6, upper_wick=0.5
    // body_pct=0.07, lower_wick/body=12, upper_wick/range=0.07
    let data = single_candle_ohlcv(25, (100.0, 101.0, 94.0, 100.5, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score > 0.0, "hammer should be positive (bullish), got {}", out.score);
    assert!((out.score - 0.60).abs() < 0.01, "hammer base score should be 0.60, got {}", out.score);
}

#[test]
fn candle_shooting_star_detected() {
    let ind = make_candle_pattern(false, false);
    // shooting star: small body at bottom, long upper wick
    // open=100.5, high=107, low=100, close=100 → body=0.5, range=7, upper_wick=6.5, lower_wick=0
    let data = single_candle_ohlcv(25, (100.5, 107.0, 100.0, 100.0, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score < 0.0, "shooting star should be negative (bearish), got {}", out.score);
    assert!((out.score - (-0.60)).abs() < 0.01, "shooting star base score should be -0.60, got {}", out.score);
}

#[test]
fn candle_hammer_body_too_large() {
    let ind = make_candle_pattern(false, false);
    // body > 30% of range → not a hammer
    // open=100, high=106, low=94, close=104 → body=4, range=12, body_pct=0.33
    let data = single_candle_ohlcv(25, (100.0, 106.0, 94.0, 104.0, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    // the candle also won't match engulfing (only 1 candle matters), so check for zero
    // note: the neutral candles preceding may also match patterns; if so, score may not be zero.
    // we use a helper that places only the target candle at the end. the neutral candles are
    // small-body drifting up, which shouldn't trigger hammer/engulfing.
    // body_pct = 4/12 = 0.33 > 0.30 max_body_pct → not hammer
    assert!(
        out.metadata.get("pattern_name").copied().unwrap_or(0.0).abs() < 0.5
            || out.score.abs() < 0.60,
        "large body should not be a hammer"
    );
}

#[test]
fn candle_hammer_wick_too_short() {
    let ind = make_candle_pattern(false, false);
    // wick not long enough: lower_wick < 2x body AND lower_wick/range < 60%
    // open=100, high=102, low=98, close=101 → body=1, range=4, lower_wick=2, upper_wick=1
    // lower_wick/body=2.0 but upper_wick/range=0.25 > max_body_pct(0.30)? No, 0.25 < 0.30
    // Actually this WOULD qualify. Let me make a case that doesn't:
    // open=100, high=102, low=98.5, close=101 → body=1, range=3.5, lower_wick=1.5, upper_wick=1
    // lower_wick/body=1.5 < 2.0, lower_wick/range=0.43 < 0.60, upper_wick/range=0.29 ≤ 0.30
    // body_pct=1/3.5=0.29 ≤ 0.30
    // Both wick checks fail → not a hammer
    let data = single_candle_ohlcv(25, (100.0, 102.0, 98.5, 101.0, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    // should not detect hammer (pattern_name should not be 2.0)
    let pn = out.metadata.get("pattern_name").copied().unwrap_or(0.0);
    assert!((pn - 2.0).abs() > 0.5, "short wick should not be hammer, pattern_name={}", pn);
}

#[test]
fn candle_hammer_zero_body_doji() {
    let ind = make_candle_pattern(false, false);
    // doji with long lower wick (body ≈ 0, wick via min_wick_pct path)
    // open=100.0, high=100.1, low=93, close=100.0 → body≈0, range=7.1, lower_wick=7.0
    // lower_wick/range = 7.0/7.1 = 0.986 >= 0.60, upper_wick/range = 0.1/7.1 = 0.014 ≤ 0.30
    let data = single_candle_ohlcv(25, (100.0, 100.1, 93.0, 100.0, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score > 0.0, "doji with long lower wick should detect as hammer, got {}", out.score);
}

#[test]
fn candle_hammer_mean_reversion() {
    let ind = make_candle_pattern(true, false); // mean_reversion = true
    // hammer is bullish → in mean reversion mode, score should be negative
    let data = single_candle_ohlcv(25, (100.0, 101.0, 94.0, 100.5, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score < 0.0, "hammer in mean reversion should be negative, got {}", out.score);
}

// ── evening star detection ──

#[test]
fn candle_evening_star_detected() {
    let ind = make_candle_pattern(false, false);
    // c1: large bullish (open=100, close=106, range=7) body_pct=6/7=0.86 >= 0.50
    // c2: doji (open=106.1, close=106.2, range=1) body_pct=0.1/1=0.10 <= 0.20
    // c3: bearish (open=105, close=102, range=4) closes at 102 <= midpoint of c1 body (103)
    let data = three_candle_ohlcv(
        25,
        (100.0, 107.0, 100.0, 106.0, 100_000.0),  // c1: large bullish
        (106.1, 107.0, 106.0, 106.2, 100_000.0),   // c2: doji
        (105.0, 106.0, 101.5, 102.0, 100_000.0),   // c3: bearish, close=102 <= midpoint 103
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score < 0.0, "evening star should be negative (bearish), got {}", out.score);
    let pn = out.metadata.get("pattern_name").copied().unwrap_or(0.0);
    assert!((pn - (-3.0)).abs() < 0.5, "pattern_name should be -3.0 (evening_star), got {}", pn);
}

#[test]
fn candle_evening_star_c2_not_doji() {
    let ind = make_candle_pattern(false, false);
    // c2 body too large (body_pct > 0.20)
    let data = three_candle_ohlcv(
        25,
        (100.0, 107.0, 100.0, 106.0, 100_000.0),  // c1: large bullish
        (104.0, 107.0, 103.0, 106.5, 100_000.0),   // c2: body=2.5, range=4, body_pct=0.63
        (105.0, 106.0, 101.5, 102.0, 100_000.0),   // c3: bearish
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    let pn = out.metadata.get("pattern_name").copied().unwrap_or(0.0);
    assert!((pn - (-3.0)).abs() > 0.5, "c2 with large body should not be evening star, pattern_name={}", pn);
}

#[test]
fn candle_evening_star_c3_not_past_midpoint() {
    let ind = make_candle_pattern(false, false);
    // c3 close above midpoint of c1 body → not evening star
    // c1 body midpoint = (100+106)/2 = 103
    let data = three_candle_ohlcv(
        25,
        (100.0, 107.0, 100.0, 106.0, 100_000.0),  // c1: large bullish
        (106.1, 107.0, 106.0, 106.2, 100_000.0),   // c2: doji
        (105.0, 106.0, 103.5, 104.0, 100_000.0),   // c3: bearish but close=104 > midpoint 103
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    let pn = out.metadata.get("pattern_name").copied().unwrap_or(0.0);
    assert!((pn - (-3.0)).abs() > 0.5, "c3 not past midpoint should not be evening star, pattern_name={}", pn);
}

#[test]
fn candle_evening_star_mean_reversion() {
    let ind = make_candle_pattern(true, false); // mean_reversion = true
    // bearish evening star → in mean reversion mode, score should be positive
    let data = three_candle_ohlcv(
        25,
        (100.0, 107.0, 100.0, 106.0, 100_000.0),
        (106.1, 107.0, 106.0, 106.2, 100_000.0),
        (105.0, 106.0, 101.5, 102.0, 100_000.0),
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score > 0.0, "evening star in mean reversion should be positive, got {}", out.score);
}

// ── confirmed engulfing (three outside up/down) detection ──

#[test]
fn candle_confirmed_engulfing_bullish() {
    let ind = make_candle_pattern(false, false);
    // c1: bearish (open=102, close=100), c2: bullish engulfing (open=99, close=103),
    // c3: bullish confirmation (close > c2 body top=103)
    let data = three_candle_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),  // c1: bearish
        (99.0, 104.0, 98.0, 103.0, 100_000.0),    // c2: bullish engulfing
        (103.0, 105.0, 102.5, 104.0, 100_000.0),  // c3: confirms, close=104 > 103
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(out.score > 0.0, "confirmed bullish engulfing should be positive, got {}", out.score);
    let pn = out.metadata.get("pattern_name").copied().unwrap_or(0.0);
    assert!((pn - 4.0).abs() < 0.5, "pattern_name should be 4.0 (three_outside_up), got {}", pn);
}

#[test]
fn candle_confirmed_engulfing_no_follow_through() {
    let ind = make_candle_pattern(false, false);
    // c1+c2 form bullish engulfing, but c3 doesn't close above c2 body top
    let data = three_candle_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),  // c1: bearish
        (99.0, 104.0, 98.0, 103.0, 100_000.0),    // c2: bullish engulfing
        (102.0, 103.5, 101.0, 102.5, 100_000.0),  // c3: close=102.5 < 103, no confirmation
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    let pn = out.metadata.get("pattern_name").copied().unwrap_or(0.0);
    // should not be confirmed engulfing (4.0), but may detect regular engulfing (1.0)
    assert!((pn - 4.0).abs() > 0.5, "no follow through should not be confirmed engulfing, pattern_name={}", pn);
}

#[test]
fn candle_confirmed_engulfing_higher_score() {
    // confirmed engulfing (0.75) should score higher than regular engulfing (0.70)
    let ind = make_candle_pattern(false, false);
    let data = three_candle_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),  // c1: bearish
        (99.0, 104.0, 98.0, 103.0, 100_000.0),    // c2: bullish engulfing
        (103.0, 105.0, 102.5, 104.0, 100_000.0),  // c3: confirms
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    assert!(
        out.score.abs() > 0.70,
        "confirmed engulfing should score higher than 0.70, got {}",
        out.score.abs()
    );
}

// ── multi-pattern integration ──

#[test]
fn candle_highest_score_wins() {
    // with decay_candles=3, set up so confirmed engulfing fires at age 0
    // and regular engulfing fires at age 1. confirmed (0.75) > engulfing (0.70*decay)
    let ind = CandlePattern::new(
        Timescale::FiveMinute,
        "cp_test".to_string(),
        0.5, 0.70,  // engulfing
        0.60, 2.0, 0.30, 0.60,  // hammer
        0.65, 0.50, 0.20,  // evening star
        0.75,  // confirmed engulfing
        false, // mean_reversion
        20, 2.0, 0.5, 5, 20,  // volume/EMA
        false, false, 0.0,  // confluence off
        3,    // scan 3 candles back
        0.8,  // 0.8 decay
    );
    let data = three_candle_ohlcv(
        25,
        (102.0, 103.0, 99.5, 100.0, 100_000.0),  // c1: bearish
        (99.0, 104.0, 98.0, 103.0, 100_000.0),    // c2: bullish engulfing
        (103.0, 105.0, 102.5, 104.0, 100_000.0),  // c3: confirms
    );
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = ind.compute(&ms).expect("should produce output");
    let pn = out.metadata.get("pattern_name").copied().unwrap_or(0.0);
    assert!((pn - 4.0).abs() < 0.5, "confirmed engulfing should win over regular, pattern_name={}", pn);
}

#[test]
fn candle_factory_new_params() {
    let registry = indicators::default_indicator_registry();
    let config = make_indicator_config(
        "candle_pattern",
        "cp_5min",
        Timescale::FiveMinute,
        0.05,
        vec![
            ("hammer_base_score", serde_json::json!(0.55)),
            ("evening_star_base_score", serde_json::json!(0.60)),
            ("confirmed_engulfing_base_score", serde_json::json!(0.80)),
            ("hammer_wick_ratio", serde_json::json!(2.5)),
            ("hammer_max_body_pct", serde_json::json!(0.25)),
            ("hammer_min_wick_pct", serde_json::json!(0.65)),
            ("evening_star_min_body_pct", serde_json::json!(0.55)),
            ("evening_star_max_doji_pct", serde_json::json!(0.15)),
        ],
    );
    let indicator = (registry.factories.get("candle_pattern").unwrap())(&config);
    assert_eq!(indicator.name(), "candle_pattern");
    // the indicator should compute successfully with custom params
    let data = single_candle_ohlcv(25, (100.0, 101.0, 94.0, 100.5, 100_000.0));
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
    let out = indicator.compute(&ms);
    assert!(out.is_some(), "factory-created indicator should compute");
}
