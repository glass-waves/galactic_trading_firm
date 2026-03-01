use std::collections::HashMap;

use ta::indicators::RelativeStrengthIndex;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct RsiIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
    overbought: f64,
    oversold: f64,
}

impl RsiIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
            overbought: 70.0,
            oversold: 30.0,
        }
    }

    pub fn with_thresholds(mut self, overbought: f64, oversold: f64) -> Self {
        self.overbought = overbought;
        self.oversold = oversold;
        self
    }

    fn normalize(&self, raw: f64) -> f64 {
        let midpoint = (self.overbought + self.oversold) / 2.0;
        let half_range = (self.overbought - self.oversold) / 2.0;
        ((raw - midpoint) / half_range).clamp(-1.0, 1.0)
    }
}

impl Indicator for RsiIndicator {
    fn name(&self) -> &str {
        "rsi"
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

        let mut rsi = RelativeStrengthIndex::new(self.period).ok()?;
        let mut last_value = None;
        for candle in candles {
            last_value = Some(rsi.next(candle.close));
        }

        let raw = last_value?;
        let score = self.normalize(raw);

        Some(IndicatorOutput {
            score,
            raw_value: raw,
            metadata: HashMap::new(),
        })
    }
}

pub fn rsi_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(14) as usize;
    let overbought = config
        .params
        .get("overbought")
        .and_then(|v| v.as_f64())
        .unwrap_or(70.0);
    let oversold = config
        .params
        .get("oversold")
        .and_then(|v| v.as_f64())
        .unwrap_or(30.0);

    Box::new(
        RsiIndicator::new(period, config.timescale, config.instance_id.clone())
            .with_thresholds(overbought, oversold),
    )
}
