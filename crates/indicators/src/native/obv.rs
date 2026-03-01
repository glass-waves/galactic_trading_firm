use std::collections::HashMap;

use ta::indicators::OnBalanceVolume;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct ObvIndicator {
    timescale: Timescale,
    instance_id: String,
    roc_period: usize,
    scale_factor: f64,
}

impl ObvIndicator {
    pub fn new(timescale: Timescale, instance_id: String) -> Self {
        Self {
            timescale,
            instance_id,
            roc_period: 10,
            scale_factor: 0.1,
        }
    }
}

impl Indicator for ObvIndicator {
    fn name(&self) -> &str {
        "obv"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.roc_period + 2
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut obv = OnBalanceVolume::new();
        let mut obv_values = Vec::with_capacity(candles.len());
        for candle in candles {
            let item: ta::DataItem = candle.into();
            obv_values.push(obv.next(&item));
        }

        let current = *obv_values.last()?;
        // rate of change of OBV over roc_period
        let lookback_idx = obv_values.len().saturating_sub(self.roc_period + 1);
        let prev = obv_values[lookback_idx];
        let roc = if prev.abs() > f64::EPSILON {
            (current - prev) / prev.abs()
        } else if current > 0.0 {
            1.0
        } else if current < 0.0 {
            -1.0
        } else {
            0.0
        };
        let score = (roc / self.scale_factor).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("obv_roc".to_string(), roc);

        Some(IndicatorOutput {
            score,
            raw_value: current,
            metadata,
        })
    }
}

pub fn obv_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    Box::new(ObvIndicator::new(
        config.timescale,
        config.instance_id.clone(),
    ))
}
