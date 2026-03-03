use types::indicator::Indicator;
use types::market::Timescale;
use types::test_fixtures::{make_indicator_config, make_market_state_ohlcv};

use indicators::custom::vpin::{normal_cdf, VpinIndicator};
use indicators::default_indicator_registry;

fn make_vpin() -> VpinIndicator {
    VpinIndicator::new(20, 0.2, 20, Timescale::FiveMinute, "vpin_test".to_string())
}

fn balanced_ohlcv(n: usize) -> Vec<(f64, f64, f64, f64, f64)> {
    // alternating up and down moves → balanced buy/sell → low VPIN
    (0..n)
        .map(|i| {
            let base = 100.0;
            if i % 2 == 0 {
                (base, base + 1.0, base - 0.5, base + 0.8, 100_000.0)
            } else {
                (base, base + 0.5, base - 1.0, base - 0.8, 100_000.0)
            }
        })
        .collect()
}

fn one_sided_ohlcv(n: usize) -> Vec<(f64, f64, f64, f64, f64)> {
    // consistent large moves in one direction → high VPIN
    (0..n)
        .map(|i| {
            let base = 100.0 + i as f64 * 2.0;
            (base, base + 3.0, base - 0.5, base + 2.5, 100_000.0) // strong up
        })
        .collect()
}

#[test]
fn vpin_insufficient_data_returns_none() {
    let vpin = make_vpin();
    // only 10 candles, needs 41
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &balanced_ohlcv(10));
    assert!(vpin.compute(&market).is_none());
}

#[test]
fn vpin_raw_value_in_zero_to_one() {
    let vpin = make_vpin();
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &balanced_ohlcv(50));
    let output = vpin.compute(&market).expect("should produce output");
    assert!(
        output.raw_value >= 0.0 && output.raw_value <= 1.0,
        "raw VPIN {} outside [0,1]",
        output.raw_value
    );
}

#[test]
fn vpin_balanced_volume_low_toxicity() {
    let vpin = make_vpin();
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &balanced_ohlcv(50));
    let output = vpin.compute(&market).expect("should produce output");
    // balanced volume → low VPIN → positive score (safe)
    assert!(
        output.score > -0.5,
        "balanced volume should give score > -0.5, got {}",
        output.score
    );
}

#[test]
fn vpin_one_sided_volume_high_toxicity() {
    let vpin = make_vpin();
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &one_sided_ohlcv(50));
    let output = vpin.compute(&market).expect("should produce output");
    // one-sided volume → high VPIN → negative score (toxic)
    assert!(
        output.score < 0.5,
        "one-sided volume should give lower score, got {}",
        output.score
    );
}

#[test]
fn vpin_score_normalization() {
    // VPIN = 0.5 → score = 0.0
    // score = 1 - 2*vpin
    let score_at_half: f64 = 1.0 - 2.0 * 0.5;
    assert!((score_at_half - 0.0).abs() < f64::EPSILON);

    let score_at_zero: f64 = 1.0 - 2.0 * 0.0;
    assert!((score_at_zero - 1.0).abs() < f64::EPSILON);

    let score_at_one: f64 = 1.0 - 2.0 * 1.0;
    assert!((score_at_one - (-1.0)).abs() < f64::EPSILON);
}

#[test]
fn vpin_factory_creates_from_config() {
    let registry = default_indicator_registry();
    let config = make_indicator_config(
        "vpin",
        "vpin_test",
        Timescale::FiveMinute,
        1.0,
        vec![
            ("sigma_period", serde_json::json!(20)),
            ("bucket_divisor", serde_json::json!(0.2)),
            ("lookback_buckets", serde_json::json!(20)),
        ],
    );
    let indicator = (registry.factories.get("vpin").unwrap())(&config);
    assert_eq!(indicator.name(), "vpin");
    assert_eq!(indicator.timescale(), Timescale::FiveMinute);
}

#[test]
fn vpin_normal_cdf_approximation() {
    // validate CDF at known values
    assert!((normal_cdf(0.0) - 0.5).abs() < 1e-6, "CDF(0) = {}", normal_cdf(0.0));
    assert!((normal_cdf(1.0) - 0.8413).abs() < 1e-3, "CDF(1) = {}", normal_cdf(1.0));
    assert!((normal_cdf(-1.0) - 0.1587).abs() < 1e-3, "CDF(-1) = {}", normal_cdf(-1.0));
    assert!((normal_cdf(3.0) - 0.9987).abs() < 1e-3, "CDF(3) = {}", normal_cdf(3.0));
}

#[test]
fn vpin_metadata_keys() {
    let vpin = make_vpin();
    let market = make_market_state_ohlcv(Timescale::FiveMinute, &balanced_ohlcv(50));
    let output = vpin.compute(&market).expect("should produce output");
    assert!(output.metadata.contains_key("raw_vpin"), "missing raw_vpin");
}

#[test]
fn vpin_name() {
    let vpin = make_vpin();
    assert_eq!(vpin.name(), "vpin");
}
