use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct DonchianIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl DonchianIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for DonchianIndicator {
    fn name(&self) -> &str {
        "donchian"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.period
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let window_start = candles.len().saturating_sub(self.period);
        let window = &candles[window_start..];

        let upper = window.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
        let lower = window.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        let close = candles.last()?.close;

        let range = upper - lower;
        if range.abs() < f64::EPSILON {
            return None;
        }

        // channel position: 2 * (close - lower) / (upper - lower) - 1
        let position = 2.0 * (close - lower) / range - 1.0;
        let score = position.clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("upper".to_string(), upper);
        metadata.insert("lower".to_string(), lower);
        metadata.insert("middle".to_string(), (upper + lower) / 2.0);

        Some(IndicatorOutput {
            score,
            raw_value: position,
            metadata,
        })
    }
}

pub fn donchian_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    Box::new(DonchianIndicator::new(period, config.timescale, config.instance_id.clone()))
}
