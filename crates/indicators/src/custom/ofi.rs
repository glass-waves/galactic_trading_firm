use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// order flow imbalance proxy indicator.
///
/// uses close location value (CLV) weighted by volume as a proxy
/// for order flow when tick-level data is unavailable.
///
/// formula:
///   CLV = ((close - low) - (high - close)) / (high - low)
///   OFI = CLV * volume
///   score = (OFI / avg_volume).clamp(-1.0, 1.0)
#[allow(dead_code)]
pub struct OfiIndicator {
    avg_period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl OfiIndicator {
    pub fn new(avg_period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            avg_period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for OfiIndicator {
    fn name(&self) -> &str {
        "ofi"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.avg_period + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let last = candles.last()?;
        let range = last.high - last.low;

        // guard against zero range (doji candle)
        let clv = if range.abs() < f64::EPSILON {
            0.0
        } else {
            ((last.close - last.low) - (last.high - last.close)) / range
        };

        let raw_ofi = clv * last.volume;

        // compute average volume over the lookback window
        let lookback_start = candles.len().saturating_sub(self.avg_period);
        let lookback_candles = &candles[lookback_start..];
        let avg_volume: f64 =
            lookback_candles.iter().map(|c| c.volume).sum::<f64>() / lookback_candles.len() as f64;

        // normalize: OFI / avg_volume, clamped to [-1, 1]
        let score = if avg_volume.abs() < f64::EPSILON {
            0.0
        } else {
            (raw_ofi / avg_volume).clamp(-1.0, 1.0)
        };

        let mut metadata = HashMap::new();
        metadata.insert("raw_ofi".to_string(), raw_ofi);
        metadata.insert("clv".to_string(), clv);
        metadata.insert("avg_volume".to_string(), avg_volume);

        Some(IndicatorOutput {
            score,
            raw_value: raw_ofi,
            metadata,
        })
    }
}

pub fn ofi_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let avg_period = config
        .params
        .get("avg_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    Box::new(OfiIndicator::new(
        avg_period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
