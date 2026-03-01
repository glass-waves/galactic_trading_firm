use std::collections::HashMap;

use ta::indicators::BollingerBands;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct BollingerBandwidthIndicator {
    period: usize,
    std_dev: f64,
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl BollingerBandwidthIndicator {
    pub fn new(period: usize, std_dev: f64, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            std_dev,
            timescale,
            instance_id,
            scale_factor: 0.1,
        }
    }
}

impl Indicator for BollingerBandwidthIndicator {
    fn name(&self) -> &str {
        "bollinger_bandwidth"
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

        let mut bb = BollingerBands::new(self.period, self.std_dev).ok()?;
        let mut last_output = None;
        for candle in candles {
            last_output = Some(bb.next(candle.close));
        }

        let output = last_output?;
        if output.average.abs() < f64::EPSILON {
            return None;
        }

        let bandwidth = (output.upper - output.lower) / output.average;
        // narrow bandwidth → squeeze (positive score = potential breakout)
        // wide bandwidth → expansion (negative score = caution)
        let score = -((bandwidth / self.scale_factor) - 1.0).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: bandwidth,
            metadata: HashMap::new(),
        })
    }
}

pub fn bollinger_bandwidth_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let std_dev = config.params.get("std_dev").and_then(|v| v.as_f64()).unwrap_or(2.0);
    Box::new(BollingerBandwidthIndicator::new(period, std_dev, config.timescale, config.instance_id.clone()))
}
