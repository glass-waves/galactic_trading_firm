use std::collections::HashMap;

use ta::indicators::BollingerBands;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct BollingerPctBIndicator {
    period: usize,
    std_dev: f64,
    timescale: Timescale,
    instance_id: String,
}

impl BollingerPctBIndicator {
    pub fn new(period: usize, std_dev: f64, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            std_dev,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for BollingerPctBIndicator {
    fn name(&self) -> &str {
        "bollinger_pct_b"
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
        let close = candles.last()?.close;
        let band_width = output.upper - output.lower;
        if band_width.abs() < f64::EPSILON {
            return None;
        }

        // %B = (close - lower) / (upper - lower), then scale to [-1, +1]
        let pct_b = (close - output.lower) / band_width;
        let score = (2.0 * pct_b - 1.0).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("pct_b".to_string(), pct_b);
        metadata.insert("upper".to_string(), output.upper);
        metadata.insert("lower".to_string(), output.lower);

        Some(IndicatorOutput {
            score,
            raw_value: pct_b,
            metadata,
        })
    }
}

pub fn bollinger_pct_b_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let std_dev = config.params.get("std_dev").and_then(|v| v.as_f64()).unwrap_or(2.0);
    Box::new(BollingerPctBIndicator::new(period, std_dev, config.timescale, config.instance_id.clone()))
}
