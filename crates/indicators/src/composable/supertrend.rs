use std::collections::HashMap;

use ta::indicators::AverageTrueRange;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct SupertrendIndicator {
    period: usize,
    multiplier: f64,
    timescale: Timescale,
    instance_id: String,
}

impl SupertrendIndicator {
    pub fn new(period: usize, multiplier: f64, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            multiplier,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for SupertrendIndicator {
    fn name(&self) -> &str {
        "supertrend"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.period + 2
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut atr = AverageTrueRange::new(self.period).ok()?;
        let mut is_bullish = true;
        let mut upper_band = f64::MAX;
        let mut lower_band = f64::MIN;

        for candle in candles {
            let item: ta::DataItem = candle.into();
            let atr_val = atr.next(&item);
            let mid = (candle.high + candle.low) / 2.0;

            let new_upper = mid + self.multiplier * atr_val;
            let new_lower = mid - self.multiplier * atr_val;

            // keep previous band if it's more favorable
            upper_band = if new_upper < upper_band || candle.close > upper_band {
                new_upper
            } else {
                upper_band
            };
            lower_band = if new_lower > lower_band || candle.close < lower_band {
                new_lower
            } else {
                lower_band
            };

            if candle.close > upper_band {
                is_bullish = true;
            } else if candle.close < lower_band {
                is_bullish = false;
            }
        }

        let close = candles.last()?.close;
        let st_level = if is_bullish { lower_band } else { upper_band };
        let distance = (close - st_level) / close;

        // ±1 direction with magnitude from distance
        let magnitude = (distance.abs() * 20.0).min(1.0);
        let score = if is_bullish { magnitude } else { -magnitude };

        let mut metadata = HashMap::new();
        metadata.insert("is_bullish".to_string(), if is_bullish { 1.0 } else { 0.0 });
        metadata.insert("supertrend_level".to_string(), st_level);

        Some(IndicatorOutput {
            score,
            raw_value: st_level,
            metadata,
        })
    }
}

pub fn supertrend_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let multiplier = config.params.get("multiplier").and_then(|v| v.as_f64()).unwrap_or(3.0);
    Box::new(SupertrendIndicator::new(period, multiplier, config.timescale, config.instance_id.clone()))
}
