use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// volume-synchronized probability of informed trading (VPIN).
///
/// estimates order flow toxicity using bulk volume classification
/// (normal CDF approximation). high VPIN indicates high probability
/// of informed trading activity (toxic flow).
///
/// score normalization: raw VPIN ∈ [0,1] mapped to [-1,+1]:
///   VPIN=0.5 → score=0.0 (neutral)
///   VPIN=0.0 → score=+1.0 (safe — low toxicity)
///   VPIN=1.0 → score=-1.0 (dangerous — high toxicity)
#[allow(dead_code)]
pub struct VpinIndicator {
    /// period for computing sigma (std dev of returns).
    sigma_period: usize,
    /// fraction of avg volume per bucket.
    bucket_divisor: f64,
    /// number of buckets in VPIN lookback window.
    lookback_buckets: usize,
    timescale: Timescale,
    instance_id: String,
}

impl VpinIndicator {
    pub fn new(
        sigma_period: usize,
        bucket_divisor: f64,
        lookback_buckets: usize,
        timescale: Timescale,
        instance_id: String,
    ) -> Self {
        Self {
            sigma_period,
            bucket_divisor,
            lookback_buckets,
            timescale,
            instance_id,
        }
    }
}

/// standard normal CDF approximation via Abramowitz-Stegun rational approximation.
/// max error ≈ 7.5e-8.
pub fn normal_cdf(x: f64) -> f64 {
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x_abs = x.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + p * x_abs);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let y = 1.0 - (a1 * t + a2 * t2 + a3 * t3 + a4 * t4 + a5 * t5) * (-x_abs * x_abs).exp();
    0.5 * (1.0 + sign * y)
}

impl Indicator for VpinIndicator {
    fn name(&self) -> &str {
        "vpin"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        // need enough candles for sigma + bucket computation
        self.sigma_period + self.lookback_buckets + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        // 1. compute sigma (std dev of log returns) over sigma_period
        let recent_start = candles.len().saturating_sub(self.sigma_period + self.lookback_buckets);
        let sigma_candles = &candles[recent_start..recent_start + self.sigma_period];
        let mut returns = Vec::with_capacity(self.sigma_period - 1);
        for i in 1..sigma_candles.len() {
            if sigma_candles[i - 1].close > 0.0 {
                returns.push((sigma_candles[i].close / sigma_candles[i - 1].close).ln());
            }
        }

        if returns.is_empty() {
            return None;
        }

        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance =
            returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
        let sigma = variance.sqrt();

        if sigma.abs() < f64::EPSILON {
            // no price movement → VPIN undefined
            return None;
        }

        // 2. compute average volume for bucket size
        let avg_volume =
            candles.iter().map(|c| c.volume).sum::<f64>() / candles.len() as f64;
        let bucket_volume = avg_volume * self.bucket_divisor;

        if bucket_volume.abs() < f64::EPSILON {
            return None;
        }

        // 3. bulk volume classification for the lookback window
        let lookback_start = candles.len().saturating_sub(self.lookback_buckets);
        let lookback_candles = &candles[lookback_start..];

        let mut buy_volume_sum = 0.0;
        let mut sell_volume_sum = 0.0;

        for candle in lookback_candles {
            if candle.volume.abs() < f64::EPSILON {
                continue;
            }
            let price_change = if candle.open > 0.0 {
                (candle.close / candle.open).ln()
            } else {
                0.0
            };

            // bulk volume classification: probability bar was buy-initiated
            let z = price_change / sigma;
            let buy_pct = normal_cdf(z);
            let sell_pct = 1.0 - buy_pct;

            buy_volume_sum += candle.volume * buy_pct;
            sell_volume_sum += candle.volume * sell_pct;
        }

        // 4. VPIN = |buy - sell| / total
        let total_volume = buy_volume_sum + sell_volume_sum;
        if total_volume.abs() < f64::EPSILON {
            return None;
        }

        let raw_vpin = (buy_volume_sum - sell_volume_sum).abs() / total_volume;

        // 5. normalize: VPIN 0.5 → 0.0, VPIN 0.0 → +1.0, VPIN 1.0 → -1.0
        let score = (1.0 - 2.0 * raw_vpin).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("raw_vpin".to_string(), raw_vpin);

        Some(IndicatorOutput {
            score,
            raw_value: raw_vpin,
            metadata,
        })
    }
}

pub fn vpin_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let sigma_period = config
        .params
        .get("sigma_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let bucket_divisor = config
        .params
        .get("bucket_divisor")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.2);
    let lookback_buckets = config
        .params
        .get("lookback_buckets")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    Box::new(VpinIndicator::new(
        sigma_period,
        bucket_divisor,
        lookback_buckets,
        config.timescale,
        config.instance_id.clone(),
    ))
}
