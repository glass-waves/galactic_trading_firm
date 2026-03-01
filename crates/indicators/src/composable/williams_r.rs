use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct WilliamsRIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl WilliamsRIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for WilliamsRIndicator {
    fn name(&self) -> &str {
        "williams_r"
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

        let highest = window.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max);
        let lowest = window.iter().map(|c| c.low).fold(f64::INFINITY, f64::min);
        let close = candles.last()?.close;

        let range = highest - lowest;
        if range.abs() < f64::EPSILON {
            return None;
        }

        // Williams %R: (highest - close) / (highest - lowest) * -100
        // range: -100 to 0
        let raw = (highest - close) / range * -100.0;
        // normalize: -100..0 → -1..+1
        let score = (raw / 50.0 + 1.0).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: raw,
            metadata: HashMap::new(),
        })
    }
}

pub fn williams_r_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
    Box::new(WilliamsRIndicator::new(period, config.timescale, config.instance_id.clone()))
}
