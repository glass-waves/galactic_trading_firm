use std::collections::HashMap;

use ta::indicators::AverageTrueRange;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct AtrIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
    neutral_atrp: f64,
    scale_factor: f64,
}

impl AtrIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
            neutral_atrp: 1.5,
            scale_factor: 1.5,
        }
    }
}

impl Indicator for AtrIndicator {
    fn name(&self) -> &str {
        "atr"
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

        let mut atr = AverageTrueRange::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            let item: ta::DataItem = candle.into();
            last_value = Some(atr.next(&item));
        }

        let atr_val = last_value?;
        let close = candles.last()?.close;
        if close.abs() < f64::EPSILON {
            return None;
        }

        // ATRP = ATR / close * 100
        let atrp = atr_val / close * 100.0;
        // score: regime centered on neutral. high ATRP = volatile (negative for caution)
        let score = -((atrp - self.neutral_atrp) / self.scale_factor).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("atrp".to_string(), atrp);

        Some(IndicatorOutput {
            score,
            raw_value: atr_val,
            metadata,
        })
    }
}

pub fn atr_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(14) as usize;
    Box::new(AtrIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
