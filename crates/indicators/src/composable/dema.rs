use std::collections::HashMap;

use ta::indicators::ExponentialMovingAverage;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// double exponential moving average: 2*EMA(close) - EMA(EMA(close))
#[allow(dead_code)]
pub struct DemaIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl DemaIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
            scale_factor: 0.02,
        }
    }
}

impl Indicator for DemaIndicator {
    fn name(&self) -> &str {
        "dema"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.period * 2
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut ema1 = ExponentialMovingAverage::new(self.period).ok()?;
        let mut ema2 = ExponentialMovingAverage::new(self.period).ok()?;

        let mut last_dema = 0.0;
        for candle in candles {
            let ema1_val = ema1.next(candle.close);
            let ema2_val = ema2.next(ema1_val);
            last_dema = 2.0 * ema1_val - ema2_val;
        }

        let close = candles.last()?.close;
        let raw = (close - last_dema) / last_dema;
        let score = (raw / self.scale_factor).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: last_dema,
            metadata: HashMap::new(),
        })
    }
}

pub fn dema_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    Box::new(DemaIndicator::new(period, config.timescale, config.instance_id.clone()))
}
