use std::collections::HashMap;

use ta::indicators::SimpleMovingAverage;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct SmaIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl SmaIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
            scale_factor: 0.02,
        }
    }
}

impl Indicator for SmaIndicator {
    fn name(&self) -> &str {
        "sma"
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

        let mut sma = SimpleMovingAverage::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            last_value = Some(sma.next(candle.close));
        }

        let sma_val = last_value?;
        let close = candles.last()?.close;
        let raw = (close - sma_val) / sma_val;
        let score = (raw / self.scale_factor).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: sma_val,
            metadata: HashMap::new(),
        })
    }
}

pub fn sma_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    Box::new(SmaIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
