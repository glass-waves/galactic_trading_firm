use std::collections::HashMap;

use ta::indicators::RateOfChange;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct RocIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl RocIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
            scale_factor: 5.0,
        }
    }
}

impl Indicator for RocIndicator {
    fn name(&self) -> &str {
        "roc"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.period + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut roc = RateOfChange::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            last_value = Some(roc.next(candle.close));
        }

        let raw = last_value?;
        // ROC is percent change; scale_factor determines how much change saturates
        let score = (raw / self.scale_factor).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: raw,
            metadata: HashMap::new(),
        })
    }
}

pub fn roc_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(12) as usize;
    Box::new(RocIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
