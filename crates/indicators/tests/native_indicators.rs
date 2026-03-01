use serde_json::json;
use types::indicator::Indicator;
use types::test_fixtures::*;
use types::Timescale;

use indicators::native::*;

// ── RSI tests ──

#[test]
fn rsi_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(5, 100.0, 1.0));
    let rsi = RsiIndicator::new(14, Timescale::FiveMinute, "rsi_14".into());
    assert!(rsi.compute(&ms).is_none());
}

#[test]
fn rsi_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let rsi = RsiIndicator::new(14, Timescale::FiveMinute, "rsi_14".into());
    let out = rsi.compute(&ms).expect("should produce output");
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn rsi_bullish_trend_positive_score() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let rsi = RsiIndicator::new(14, Timescale::FiveMinute, "rsi_14".into());
    let out = rsi.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
}

#[test]
fn rsi_bearish_trend_negative_score() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_down(30, 200.0, 1.0));
    let rsi = RsiIndicator::new(14, Timescale::FiveMinute, "rsi_14".into());
    let out = rsi.compute(&ms).unwrap();
    assert!(out.score < 0.0, "expected negative, got {}", out.score);
}

#[test]
fn rsi_raw_value_in_0_to_100() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 0.5));
    let rsi = RsiIndicator::new(14, Timescale::FiveMinute, "rsi_14".into());
    let out = rsi.compute(&ms).unwrap();
    assert!(out.raw_value >= 0.0 && out.raw_value <= 100.0);
}

#[test]
fn rsi_name_and_timescale() {
    let rsi = RsiIndicator::new(14, Timescale::OneHour, "rsi_1h".into());
    assert_eq!(rsi.name(), "rsi");
    assert_eq!(rsi.timescale(), Timescale::OneHour);
}

#[test]
fn rsi_min_lookback() {
    let rsi = RsiIndicator::new(14, Timescale::FiveMinute, "rsi_14".into());
    assert!(rsi.min_lookback() >= 14);
}

#[test]
fn rsi_factory_creates_from_config() {
    let cfg = make_indicator_config("rsi", "rsi_14", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]);
    let ind = rsi_factory(&cfg);
    assert_eq!(ind.name(), "rsi");
    assert_eq!(ind.timescale(), Timescale::FiveMinute);
}

#[test]
fn rsi_custom_thresholds_affect_normalization() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 0.5));
    let rsi_default = RsiIndicator::new(14, Timescale::FiveMinute, "d".into());
    let rsi_custom = RsiIndicator::new(14, Timescale::FiveMinute, "c".into())
        .with_thresholds(80.0, 20.0);
    let out_d = rsi_default.compute(&ms).unwrap();
    let out_c = rsi_custom.compute(&ms).unwrap();
    assert!((out_d.raw_value - out_c.raw_value).abs() < f64::EPSILON);
    assert!(out_c.score.abs() <= out_d.score.abs() + f64::EPSILON);
}

// ── EMA tests ──

#[test]
fn ema_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(5, 100.0, 1.0));
    let ema = EmaIndicator::new(20, Timescale::FiveMinute, "ema_20".into());
    assert!(ema.compute(&ms).is_none());
}

#[test]
fn ema_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let ema = EmaIndicator::new(10, Timescale::FiveMinute, "ema_10".into());
    let out = ema.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn ema_bullish_trend_positive_score() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let ema = EmaIndicator::new(10, Timescale::FiveMinute, "ema_10".into());
    let out = ema.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
}

#[test]
fn ema_factory_works() {
    let cfg = make_indicator_config("ema", "ema_20", Timescale::FiveMinute, 1.0, vec![("period", json!(20))]);
    let ind = ema_factory(&cfg);
    assert_eq!(ind.name(), "ema");
}

// ── SMA tests ──

#[test]
fn sma_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(5, 100.0, 1.0));
    let sma = SmaIndicator::new(20, Timescale::FiveMinute, "sma_20".into());
    assert!(sma.compute(&ms).is_none());
}

#[test]
fn sma_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let sma = SmaIndicator::new(10, Timescale::FiveMinute, "sma_10".into());
    let out = sma.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn sma_bullish_trend_positive_score() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let sma = SmaIndicator::new(10, Timescale::FiveMinute, "sma_10".into());
    let out = sma.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
}

#[test]
fn sma_factory_works() {
    let cfg = make_indicator_config("sma", "sma_20", Timescale::FiveMinute, 1.0, vec![("period", json!(20))]);
    let ind = sma_factory(&cfg);
    assert_eq!(ind.name(), "sma");
}

// ── MACD tests ──

#[test]
fn macd_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(20, 100.0, 1.0));
    let macd = MacdIndicator::new(12, 26, 9, Timescale::FiveMinute, "macd".into());
    assert!(macd.compute(&ms).is_none());
}

#[test]
fn macd_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let macd = MacdIndicator::new(12, 26, 9, Timescale::FiveMinute, "macd".into());
    let out = macd.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn macd_bullish_histogram_positive_score() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(50, 100.0, 1.0));
    let macd = MacdIndicator::new(12, 26, 9, Timescale::FiveMinute, "macd".into());
    let out = macd.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive histogram score, got {}", out.score);
    assert!(out.metadata.contains_key("histogram"));
}

#[test]
fn macd_factory_works() {
    let cfg = make_indicator_config("macd", "macd_d", Timescale::FiveMinute, 1.0, vec![]);
    let ind = macd_factory(&cfg);
    assert_eq!(ind.name(), "macd");
}

// ── Bollinger tests ──

#[test]
fn bollinger_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(10, 100.0, 1.0));
    let bb = BollingerIndicator::new(20, 2.0, Timescale::FiveMinute, "bb".into());
    assert!(bb.compute(&ms).is_none());
}

#[test]
fn bollinger_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(50, 100.0, 3.0));
    let bb = BollingerIndicator::new(20, 2.0, Timescale::FiveMinute, "bb".into());
    let out = bb.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn bollinger_bullish_trend_positive_score() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let bb = BollingerIndicator::new(20, 2.0, Timescale::FiveMinute, "bb".into());
    let out = bb.compute(&ms).unwrap();
    // in a strong uptrend, price is above middle band → positive %B
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
    assert!(out.metadata.contains_key("upper"));
    assert!(out.metadata.contains_key("lower"));
}

#[test]
fn bollinger_factory_works() {
    let cfg = make_indicator_config("bollinger", "bb", Timescale::FiveMinute, 1.0, vec![("period", json!(20)), ("std_dev", json!(2.0))]);
    let ind = bollinger_factory(&cfg);
    assert_eq!(ind.name(), "bollinger");
}

// ── ATR tests ──

#[test]
fn atr_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(5, 100.0, 1.0));
    let atr = AtrIndicator::new(14, Timescale::FiveMinute, "atr_14".into());
    assert!(atr.compute(&ms).is_none());
}

#[test]
fn atr_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 0.5));
    let atr = AtrIndicator::new(14, Timescale::FiveMinute, "atr_14".into());
    let out = atr.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn atr_raw_value_always_non_negative() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_down_ohlcv(30, 200.0, 1.0));
    let atr = AtrIndicator::new(14, Timescale::FiveMinute, "atr_14".into());
    let out = atr.compute(&ms).unwrap();
    assert!(out.raw_value >= 0.0, "ATR must be non-negative, got {}", out.raw_value);
}

#[test]
fn atr_factory_works() {
    let cfg = make_indicator_config("atr", "atr_14", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]);
    let ind = atr_factory(&cfg);
    assert_eq!(ind.name(), "atr");
}

// ── Keltner tests ──

#[test]
fn keltner_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(10, 100.0, 1.0));
    let kc = KeltnerIndicator::new(20, 2.0, Timescale::FiveMinute, "kc".into());
    assert!(kc.compute(&ms).is_none());
}

#[test]
fn keltner_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(40, 100.0, 0.5));
    let kc = KeltnerIndicator::new(20, 2.0, Timescale::FiveMinute, "kc".into());
    let out = kc.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn keltner_bullish_trend_positive_score() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(40, 100.0, 1.0));
    let kc = KeltnerIndicator::new(20, 2.0, Timescale::FiveMinute, "kc".into());
    let out = kc.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
}

#[test]
fn keltner_factory_works() {
    let cfg = make_indicator_config("keltner", "kc", Timescale::FiveMinute, 1.0, vec![("period", json!(20))]);
    let ind = keltner_factory(&cfg);
    assert_eq!(ind.name(), "keltner");
}

// ── fast stochastic tests ──

#[test]
fn stochastic_fast_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(5, 100.0, 1.0));
    let stoch = FastStochasticIndicator::new(14, Timescale::FiveMinute, "sf".into());
    assert!(stoch.compute(&ms).is_none());
}

#[test]
fn stochastic_fast_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &ranging_ohlcv(30, 100.0, 3.0));
    let stoch = FastStochasticIndicator::new(14, Timescale::FiveMinute, "sf".into());
    let out = stoch.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn stochastic_fast_bullish_trend_positive() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 1.0));
    let stoch = FastStochasticIndicator::new(14, Timescale::FiveMinute, "sf".into());
    let out = stoch.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
}

#[test]
fn stochastic_fast_factory_works() {
    let cfg = make_indicator_config("stochastic_fast", "sf", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]);
    let ind = fast_stochastic_factory(&cfg);
    assert_eq!(ind.name(), "stochastic_fast");
}

// ── slow stochastic tests ──

#[test]
fn stochastic_slow_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(5, 100.0, 1.0));
    let stoch = SlowStochasticIndicator::new(14, 3, Timescale::FiveMinute, "ss".into());
    assert!(stoch.compute(&ms).is_none());
}

#[test]
fn stochastic_slow_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &ranging_ohlcv(30, 100.0, 3.0));
    let stoch = SlowStochasticIndicator::new(14, 3, Timescale::FiveMinute, "ss".into());
    let out = stoch.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn stochastic_slow_factory_works() {
    let cfg = make_indicator_config("stochastic_slow", "ss", Timescale::FiveMinute, 1.0, vec![("stochastic_period", json!(14)), ("ema_period", json!(3))]);
    let ind = slow_stochastic_factory(&cfg);
    assert_eq!(ind.name(), "stochastic_slow");
}

// ── CCI tests ──

#[test]
fn cci_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(10, 100.0, 1.0));
    let cci = CciIndicator::new(20, Timescale::FiveMinute, "cci".into());
    assert!(cci.compute(&ms).is_none());
}

#[test]
fn cci_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &ranging_ohlcv(40, 100.0, 3.0));
    let cci = CciIndicator::new(20, Timescale::FiveMinute, "cci".into());
    let out = cci.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn cci_bullish_trend_positive_score() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(40, 100.0, 1.0));
    let cci = CciIndicator::new(20, Timescale::FiveMinute, "cci".into());
    let out = cci.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
}

#[test]
fn cci_factory_works() {
    let cfg = make_indicator_config("cci", "cci_20", Timescale::FiveMinute, 1.0, vec![("period", json!(20))]);
    let ind = cci_factory(&cfg);
    assert_eq!(ind.name(), "cci");
}

// ── MFI tests ──

#[test]
fn mfi_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(5, 100.0, 1.0));
    let mfi = MfiIndicator::new(14, Timescale::FiveMinute, "mfi".into());
    assert!(mfi.compute(&ms).is_none());
}

#[test]
fn mfi_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 0.5));
    let mfi = MfiIndicator::new(14, Timescale::FiveMinute, "mfi".into());
    let out = mfi.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn mfi_factory_works() {
    let cfg = make_indicator_config("mfi", "mfi_14", Timescale::FiveMinute, 1.0, vec![("period", json!(14))]);
    let ind = mfi_factory(&cfg);
    assert_eq!(ind.name(), "mfi");
}

// ── ROC tests ──

#[test]
fn roc_insufficient_data_returns_none() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(5, 100.0, 1.0));
    let roc = RocIndicator::new(12, Timescale::FiveMinute, "roc".into());
    assert!(roc.compute(&ms).is_none());
}

#[test]
fn roc_score_in_range() {
    let ms = make_market_state(Timescale::FiveMinute, &ranging(30, 100.0, 3.0));
    let roc = RocIndicator::new(12, Timescale::FiveMinute, "roc".into());
    let out = roc.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn roc_bullish_trend_positive_score() {
    let ms = make_market_state(Timescale::FiveMinute, &trending_up(30, 100.0, 1.0));
    let roc = RocIndicator::new(12, Timescale::FiveMinute, "roc".into());
    let out = roc.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive, got {}", out.score);
}

#[test]
fn roc_factory_works() {
    let cfg = make_indicator_config("roc", "roc_12", Timescale::FiveMinute, 1.0, vec![("period", json!(12))]);
    let ind = roc_factory(&cfg);
    assert_eq!(ind.name(), "roc");
}

// ── OBV tests ──

#[test]
fn obv_insufficient_data_returns_none() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(5, 100.0, 1.0));
    let obv = ObvIndicator::new(Timescale::FiveMinute, "obv".into());
    assert!(obv.compute(&ms).is_none());
}

#[test]
fn obv_score_in_range() {
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 0.5));
    let obv = ObvIndicator::new(Timescale::FiveMinute, "obv".into());
    let out = obv.compute(&ms).unwrap();
    assert!(out.score >= -1.0 && out.score <= 1.0);
}

#[test]
fn obv_bullish_volume_direction() {
    // trending up → OBV should be increasing → positive ROC → positive score
    let ms = make_market_state_ohlcv(Timescale::FiveMinute, &trending_up_ohlcv(30, 100.0, 1.0));
    let obv = ObvIndicator::new(Timescale::FiveMinute, "obv".into());
    let out = obv.compute(&ms).unwrap();
    assert!(out.score > 0.0, "expected positive OBV score, got {}", out.score);
}

#[test]
fn obv_factory_works() {
    let cfg = make_indicator_config("obv", "obv_1", Timescale::FiveMinute, 1.0, vec![]);
    let ind = obv_factory(&cfg);
    assert_eq!(ind.name(), "obv");
}
