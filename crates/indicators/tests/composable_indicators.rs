use serde_json::json;
use types::indicator::Indicator;
use types::test_fixtures::*;
use types::Timescale;

use indicators::composable::*;

// ── Bollinger %B tests ──

#[test]
fn bollinger_pct_b_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(10, 100.0, 1.0));
    let ind = BollingerPctBIndicator::new(20, 2.0, Timescale::FiveMinute, "bb_pctb".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn bollinger_pct_b_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let ind = BollingerPctBIndicator::new(20, 2.0, Timescale::FiveMinute, "bb_pctb".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn bollinger_pct_b_upper_band_positive() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let ind = BollingerPctBIndicator::new(20, 2.0, Timescale::FiveMinute, "bb_pctb".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score > 0.0, "near upper band should be positive, got {}", out.score);
}

#[test]
fn bollinger_pct_b_lower_band_negative() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_down(30, 200.0, 1.0));
    let ind = BollingerPctBIndicator::new(20, 2.0, Timescale::FiveMinute, "bb_pctb".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score < 0.0, "near lower band should be negative, got {}", out.score);
}

#[test]
fn bollinger_pct_b_factory_works() {
    let cfg = make_indicator_config("bollinger_pct_b", "bb_pctb", Timescale::FiveMinute, 1.0, vec![]);
    let ind = bollinger_pct_b_factory(&cfg);
    assert_eq!(ind.name(), "bollinger_pct_b");
}

// ── Bollinger Bandwidth tests ──

#[test]
fn bollinger_bandwidth_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let ind = BollingerBandwidthIndicator::new(20, 2.0, Timescale::FiveMinute, "bb_bw".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn bollinger_bandwidth_factory_works() {
    let cfg = make_indicator_config("bollinger_bandwidth", "bb_bw", Timescale::FiveMinute, 1.0, vec![]);
    let ind = bollinger_bandwidth_factory(&cfg);
    assert_eq!(ind.name(), "bollinger_bandwidth");
}

// ── ADX tests ──

#[test]
fn adx_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(10, 100.0, 1.0));
    let ind = AdxIndicator::new(14, Timescale::FiveMinute, "adx".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn adx_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(50, 100.0, 1.0));
    let ind = AdxIndicator::new(14, Timescale::FiveMinute, "adx".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn adx_raw_non_negative() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(50, 100.0, 1.0));
    let ind = AdxIndicator::new(14, Timescale::FiveMinute, "adx".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.raw_value >= 0.0, "ADX should be non-negative, got {}", out.raw_value);
}

#[test]
fn adx_factory_works() {
    let cfg = make_indicator_config("adx", "adx_14", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]);
    let ind = adx_factory(&cfg);
    assert_eq!(ind.name(), "adx");
}

// ── Supertrend tests ──

#[test]
fn supertrend_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(5, 100.0, 1.0));
    let ind = SupertrendIndicator::new(10, 3.0, Timescale::FiveMinute, "st".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn supertrend_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 0.5));
    let ind = SupertrendIndicator::new(10, 3.0, Timescale::FiveMinute, "st".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn supertrend_bullish_trend_positive() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(40, 100.0, 1.0));
    let ind = SupertrendIndicator::new(10, 3.0, Timescale::FiveMinute, "st".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive for bullish, got {}", out.score);
    assert!((out.metadata["is_bullish"] - 1.0).abs() < f64::EPSILON);
}

#[test]
fn supertrend_factory_works() {
    let cfg = make_indicator_config("supertrend", "st_1", Timescale::FiveMinute, 1.0, vec![]);
    let ind = supertrend_factory(&cfg);
    assert_eq!(ind.name(), "supertrend");
}

// ── VWAP Distance tests ──

#[test]
fn vwap_distance_above_positive() {
    let mut ms = make_market_state(Timescale::FiveMinute, &[100.0]);
    ms.session_vwap = 99.0;
    ms.last_price = 100.0;
    let ind = VwapDistanceIndicator::new(Timescale::FiveMinute, "vwap".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score > 0.0, "price above VWAP should be positive, got {}", out.score);
}

#[test]
fn vwap_distance_below_negative() {
    let mut ms = make_market_state(Timescale::FiveMinute, &[98.0]);
    ms.session_vwap = 100.0;
    ms.last_price = 98.0;
    let ind = VwapDistanceIndicator::new(Timescale::FiveMinute, "vwap".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score < 0.0, "price below VWAP should be negative, got {}", out.score);
}

#[test]
fn vwap_distance_score_in_range() {
    let mut ms = make_market_state(Timescale::FiveMinute, &[110.0]);
    ms.session_vwap = 100.0;
    ms.last_price = 110.0;
    let ind = VwapDistanceIndicator::new(Timescale::FiveMinute, "vwap".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn vwap_distance_factory_works() {
    let cfg = make_indicator_config("vwap_distance", "vwap_d", Timescale::FiveMinute, 1.0, vec![]);
    let ind = vwap_distance_factory(&cfg);
    assert_eq!(ind.name(), "vwap_distance");
}

// ── Stochastic RSI tests ──

#[test]
fn stochastic_rsi_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(10, 100.0, 1.0));
    let ind = StochasticRsiIndicator::new(14, 14, Timescale::FiveMinute, "stochrsi".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn stochastic_rsi_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let ind = StochasticRsiIndicator::new(14, 14, Timescale::FiveMinute, "stochrsi".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn stochastic_rsi_factory_works() {
    let cfg = make_indicator_config("stochastic_rsi", "sr", Timescale::FiveMinute, 1.0, vec![]);
    let ind = stochastic_rsi_factory(&cfg);
    assert_eq!(ind.name(), "stochastic_rsi");
}

// ── Williams %R tests ──

#[test]
fn williams_r_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(5, 100.0, 1.0));
    let ind = WilliamsRIndicator::new(14, Timescale::FiveMinute, "wr".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn williams_r_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(30, 100.0, 3.0));
    let ind = WilliamsRIndicator::new(14, Timescale::FiveMinute, "wr".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn williams_r_bullish_positive() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let ind = WilliamsRIndicator::new(14, Timescale::FiveMinute, "wr".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive for uptrend, got {}", out.score);
}

#[test]
fn williams_r_factory_works() {
    let cfg = make_indicator_config("williams_r", "wr_14", Timescale::FiveMinute, 1.0, vec![]);
    let ind = williams_r_factory(&cfg);
    assert_eq!(ind.name(), "williams_r");
}

// ── Donchian tests ──

#[test]
fn donchian_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(10, 100.0, 1.0));
    let ind = DonchianIndicator::new(20, Timescale::FiveMinute, "dc".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn donchian_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(30, 100.0, 3.0));
    let ind = DonchianIndicator::new(20, Timescale::FiveMinute, "dc".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn donchian_bullish_positive() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let ind = DonchianIndicator::new(20, Timescale::FiveMinute, "dc".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive for uptrend, got {}", out.score);
}

#[test]
fn donchian_factory_works() {
    let cfg = make_indicator_config("donchian", "dc_20", Timescale::FiveMinute, 1.0, vec![]);
    let ind = donchian_factory(&cfg);
    assert_eq!(ind.name(), "donchian");
}

// ── DEMA tests ──

#[test]
fn dema_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(10, 100.0, 1.0));
    let ind = DemaIndicator::new(20, Timescale::FiveMinute, "dema".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn dema_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let ind = DemaIndicator::new(10, Timescale::FiveMinute, "dema".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn dema_bullish_positive() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(50, 100.0, 1.0));
    let ind = DemaIndicator::new(10, Timescale::FiveMinute, "dema".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive for uptrend, got {}", out.score);
}

#[test]
fn dema_factory_works() {
    let cfg = make_indicator_config("dema", "dema_20", Timescale::FiveMinute, 1.0, vec![]);
    let ind = dema_factory(&cfg);
    assert_eq!(ind.name(), "dema");
}

// ── TTM Squeeze tests ──

#[test]
fn ttm_squeeze_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(10, 100.0, 1.0));
    let ind = TtmSqueezeIndicator::new(20, 2.0, 1.5, Timescale::FiveMinute, "ttm".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn ttm_squeeze_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(40, 100.0, 0.5));
    let ind = TtmSqueezeIndicator::new(20, 2.0, 1.5, Timescale::FiveMinute, "ttm".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn ttm_squeeze_has_metadata() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &ranging_ohlcv(40, 100.0, 2.0));
    let ind = TtmSqueezeIndicator::new(20, 2.0, 1.5, Timescale::FiveMinute, "ttm".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.metadata.contains_key("squeeze_on"));
    assert!(out.metadata.contains_key("momentum"));
}

#[test]
fn ttm_squeeze_factory_works() {
    let cfg = make_indicator_config("ttm_squeeze", "ttm_1", Timescale::FiveMinute, 1.0, vec![]);
    let ind = ttm_squeeze_factory(&cfg);
    assert_eq!(ind.name(), "ttm_squeeze");
}

// ── Awesome Oscillator tests ──

#[test]
fn ao_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(20, 100.0, 1.0));
    let ind = AwesomeOscillatorIndicator::new(5, 34, Timescale::FiveMinute, "ao".into());
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn ao_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let ind = AwesomeOscillatorIndicator::new(5, 34, Timescale::FiveMinute, "ao".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn ao_bullish_positive() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(50, 100.0, 1.0));
    let ind = AwesomeOscillatorIndicator::new(5, 34, Timescale::FiveMinute, "ao".into());
    let out = ind.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive for uptrend, got {}", out.score);
}

#[test]
fn ao_factory_works() {
    let cfg = make_indicator_config("awesome_oscillator", "ao_1", Timescale::FiveMinute, 1.0, vec![]);
    let ind = awesome_oscillator_factory(&cfg);
    assert_eq!(ind.name(), "awesome_oscillator");
}

// ── MomentumPersistence ──

use indicators::composable::momentum_persistence::{
    momentum_persistence_factory, MomentumPersistenceIndicator,
};

#[test]
fn momentum_persistence_accelerating_positive() {
    let ind = MomentumPersistenceIndicator::new(5, Timescale::FiveMinute, "mp_5".into());
    // steadily accelerating trend
    let prices: Vec<f64> = (0..25)
        .map(|i| 100.0 + (i as f64).powi(2) * 0.1) // quadratic acceleration
        .collect();
    let ms = make_market_state(Timescale::FiveMinute, &prices);
    let out = ind.compute(&ms).expect("should compute");
    assert!(out.score > 0.0, "accelerating should give positive, got {}", out.score);
}

#[test]
fn momentum_persistence_insufficient_data_none() {
    let ind = MomentumPersistenceIndicator::new(10, Timescale::FiveMinute, "mp_10".into());
    let ms = make_market_state(Timescale::FiveMinute, &[100.0; 5]);
    assert!(ind.compute(&ms).is_none());
}

#[test]
fn momentum_persistence_factory_works() {
    let cfg = make_indicator_config("momentum_persistence", "mp_1", Timescale::FiveMinute, 1.0, vec![]);
    let ind = momentum_persistence_factory(&cfg);
    assert_eq!(ind.name(), "momentum_persistence");
}
