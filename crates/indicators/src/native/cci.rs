use std::collections::HashMap;

use ta::indicators::CommodityChannelIndex;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct CciIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl CciIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for CciIndicator {
    fn name(&self) -> &str {
        "cci"
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

        let mut cci = CommodityChannelIndex::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            let item: ta::DataItem = candle.into();
            last_value = Some(cci.next(&item));
        }

        let raw = last_value?;
        // CCI is unbounded; ±200 is typical extreme range
        let score = (raw / 200.0).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: raw,
            metadata: HashMap::new(),
        })
    }
}

pub fn cci_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    Box::new(CciIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
