use std::collections::HashMap;

use ta::indicators::MoneyFlowIndex;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct MfiIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl MfiIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for MfiIndicator {
    fn name(&self) -> &str {
        "mfi"
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

        let mut mfi = MoneyFlowIndex::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            let item: ta::DataItem = candle.into();
            last_value = Some(mfi.next(&item));
        }

        let raw = last_value?;
        // MFI is 0-100, same normalization as RSI
        let score = ((raw - 50.0) / 20.0).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: raw,
            metadata: HashMap::new(),
        })
    }
}

pub fn mfi_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(14) as usize;
    Box::new(MfiIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
