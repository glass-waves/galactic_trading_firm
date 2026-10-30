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
    /// `standard: true` runs the textbook supertrend (bands carried from the PREVIOUS bar, trend
    /// flips when the close crosses the carried band). the default (`false`) keeps the legacy
    /// computation every promoted config was fitted on, which — because it re-tightens the bands
    /// with the current close before comparing — never turns bearish (audit 2026-10-29).
    standard: bool,
}

impl SupertrendIndicator {
    pub fn new(period: usize, multiplier: f64, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            multiplier,
            timescale,
            instance_id,
            standard: false,
        }
    }

    pub fn with_standard(mut self, standard: bool) -> Self {
        self.standard = standard;
        self
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
        let mut prev_close: Option<f64> = None;

        for candle in candles {
            let item: ta::DataItem = candle.into();
            let atr_val = atr.next(&item);
            let mid = (candle.high + candle.low) / 2.0;

            let new_upper = mid + self.multiplier * atr_val;
            let new_lower = mid - self.multiplier * atr_val;

            if self.standard {
                // carry the band unless it tightens, or the PREVIOUS close already broke it
                let pc = prev_close.unwrap_or(candle.close);
                upper_band = if new_upper < upper_band || pc > upper_band { new_upper } else { upper_band };
                lower_band = if new_lower > lower_band || pc < lower_band { new_lower } else { lower_band };
                // the trend holds until the close crosses the band that supports it
                if is_bullish && candle.close < lower_band {
                    is_bullish = false;
                } else if !is_bullish && candle.close > upper_band {
                    is_bullish = true;
                }
                prev_close = Some(candle.close);
                continue;
            }

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
    let standard = config.params.get("standard").and_then(|v| v.as_bool()).unwrap_or(false);
    Box::new(SupertrendIndicator::new(period, multiplier, config.timescale, config.instance_id.clone()).with_standard(standard))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn market(closes: &[f64]) -> MarketState {
        types::test_fixtures::make_market_state(Timescale::OneHour, closes)
    }

    #[test]
    fn legacy_never_turns_bearish_on_a_collapse() {
        let closes: Vec<f64> = (0..40).map(|i| 100.0 - i as f64 * 2.0).collect();
        let out = SupertrendIndicator::new(10, 3.0, Timescale::OneHour, "st".into()).compute(&market(&closes)).unwrap();
        assert_eq!(out.metadata["is_bullish"], 1.0, "the legacy computation stays bullish (documented defect)");
    }

    #[test]
    fn standard_turns_bearish_on_a_collapse_and_bullish_on_a_rally() {
        let down: Vec<f64> = (0..40).map(|i| 100.0 - i as f64 * 2.0).collect();
        let out = SupertrendIndicator::new(10, 3.0, Timescale::OneHour, "st".into()).with_standard(true).compute(&market(&down)).unwrap();
        assert_eq!(out.metadata["is_bullish"], 0.0);
        assert!(out.score < 0.0);
        let up: Vec<f64> = (0..40).map(|i| 20.0 + i as f64 * 2.0).collect();
        let out = SupertrendIndicator::new(10, 3.0, Timescale::OneHour, "st".into()).with_standard(true).compute(&market(&up)).unwrap();
        assert_eq!(out.metadata["is_bullish"], 1.0);
        assert!(out.score > 0.0);
    }
}
