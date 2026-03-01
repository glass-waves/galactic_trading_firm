use std::collections::HashMap;

use ta::indicators::KeltnerChannel;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct KeltnerIndicator {
    period: usize,
    multiplier: f64,
    timescale: Timescale,
    instance_id: String,
}

impl KeltnerIndicator {
    pub fn new(period: usize, multiplier: f64, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            multiplier,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for KeltnerIndicator {
    fn name(&self) -> &str {
        "keltner"
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

        let mut kc = KeltnerChannel::new(self.period, self.multiplier).ok()?;
        let mut last_output = None;
        for candle in candles {
            let item: ta::DataItem = candle.into();
            last_output = Some(kc.next(&item));
        }

        let output = last_output?;
        let close = candles.last()?.close;
        let band_width = output.upper - output.lower;
        if band_width.abs() < f64::EPSILON {
            return None;
        }

        // channel position like bollinger: 2 * (close - lower) / (upper - lower) - 1
        let position = 2.0 * (close - output.lower) / band_width - 1.0;
        let score = position.clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("upper".to_string(), output.upper);
        metadata.insert("lower".to_string(), output.lower);
        metadata.insert("average".to_string(), output.average);

        Some(IndicatorOutput {
            score,
            raw_value: position,
            metadata,
        })
    }
}

pub fn keltner_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let multiplier = config
        .params
        .get("multiplier")
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);
    Box::new(KeltnerIndicator::new(
        period,
        multiplier,
        config.timescale,
        config.instance_id.clone(),
    ))
}
