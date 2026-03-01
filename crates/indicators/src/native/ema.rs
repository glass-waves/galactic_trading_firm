use std::collections::HashMap;

use ta::indicators::ExponentialMovingAverage;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct EmaIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl EmaIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
            scale_factor: 0.02,
        }
    }
}

impl Indicator for EmaIndicator {
    fn name(&self) -> &str {
        "ema"
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

        let mut ema = ExponentialMovingAverage::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            last_value = Some(ema.next(candle.close));
        }

        let ema_val = last_value?;
        let close = candles.last()?.close;
        let raw = (close - ema_val) / ema_val;
        let score = (raw / self.scale_factor).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: ema_val,
            metadata: HashMap::new(),
        })
    }
}

pub fn ema_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    Box::new(EmaIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
