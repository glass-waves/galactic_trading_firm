use std::collections::HashMap;

use ta::indicators::AverageTrueRange;
use ta::indicators::ExponentialMovingAverage;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// approximate ADX using smoothed directional movement over ATR.
/// computes +DI and -DI from candle sequences, then ADX from their smoothed ratio.
#[allow(dead_code)]
pub struct AdxIndicator {
    period: usize,
    timescale: Timescale,
    instance_id: String,
    /// `signed: true` gives the trend strength the SIGN of (+DI − −DI), so a strong down-trend
    /// scores negative. the default (`false`) is the legacy non-directional `adx/50 − 1`, which adds
    /// a direction-free offset to any directional sum it sits in (audit 2026-10-29).
    signed: bool,
}

impl AdxIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
            signed: false,
        }
    }

    pub fn with_signed(mut self, signed: bool) -> Self {
        self.signed = signed;
        self
    }
}

impl Indicator for AdxIndicator {
    fn name(&self) -> &str {
        "adx"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.period * 2 + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut atr = AverageTrueRange::new(self.period).ok()?;
        let mut plus_dm_ema = ExponentialMovingAverage::new(self.period).ok()?;
        let mut minus_dm_ema = ExponentialMovingAverage::new(self.period).ok()?;
        let mut dx_ema = ExponentialMovingAverage::new(self.period).ok()?;

        let mut last_adx = 0.0;
        let mut last_plus_di = 0.0;
        let mut last_minus_di = 0.0;

        for i in 0..candles.len() {
            let item: ta::DataItem = (&candles[i]).into();
            let atr_val = atr.next(&item);

            if i == 0 {
                continue;
            }

            let up_move = candles[i].high - candles[i - 1].high;
            let down_move = candles[i - 1].low - candles[i].low;

            let plus_dm = if up_move > down_move && up_move > 0.0 { up_move } else { 0.0 };
            let minus_dm = if down_move > up_move && down_move > 0.0 { down_move } else { 0.0 };

            let smooth_plus = plus_dm_ema.next(plus_dm);
            let smooth_minus = minus_dm_ema.next(minus_dm);

            if atr_val.abs() < f64::EPSILON {
                continue;
            }

            last_plus_di = (smooth_plus / atr_val) * 100.0;
            last_minus_di = (smooth_minus / atr_val) * 100.0;

            let di_sum = last_plus_di + last_minus_di;
            let dx = if di_sum.abs() > f64::EPSILON {
                ((last_plus_di - last_minus_di).abs() / di_sum) * 100.0
            } else {
                0.0
            };

            last_adx = dx_ema.next(dx);
        }

        // ADX ranges 0-100. score = adx/50 - 1.0
        // ADX > 50 → strong trend → positive score
        // ADX < 50 → weak trend → negative score
        let score = if self.signed {
            // direction from the DI balance, magnitude from the trend strength (ADX 50 = full)
            let di_sum = last_plus_di + last_minus_di;
            let direction = if di_sum.abs() > f64::EPSILON { (last_plus_di - last_minus_di) / di_sum } else { 0.0 };
            (direction * (last_adx / 50.0).min(1.0)).clamp(-1.0, 1.0)
        } else {
            (last_adx / 50.0 - 1.0).clamp(-1.0, 1.0)
        };

        let mut metadata = HashMap::new();
        metadata.insert("plus_di".to_string(), last_plus_di);
        metadata.insert("minus_di".to_string(), last_minus_di);

        Some(IndicatorOutput {
            score,
            raw_value: last_adx,
            metadata,
        })
    }
}

pub fn adx_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config.params.get("period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
    let signed = config.params.get("signed").and_then(|v| v.as_bool()).unwrap_or(false);
    Box::new(AdxIndicator::new(period, config.timescale, config.instance_id.clone()).with_signed(signed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn market(closes: &[f64]) -> MarketState {
        types::test_fixtures::make_market_state(Timescale::OneHour, closes)
    }

    #[test]
    fn signed_score_follows_the_direction_of_a_strong_trend() {
        let down: Vec<f64> = (0..40).map(|i| 100.0 - i as f64 * 2.0).collect();
        let legacy = AdxIndicator::new(14, Timescale::OneHour, "adx".into()).compute(&market(&down)).unwrap();
        let signed = AdxIndicator::new(14, Timescale::OneHour, "adx".into()).with_signed(true).compute(&market(&down)).unwrap();
        assert!(legacy.score > 0.0, "legacy: a strong trend scores positive whatever its direction");
        assert!(signed.score < 0.0, "signed: a strong DOWN-trend scores negative");
        let up: Vec<f64> = (0..40).map(|i| 20.0 + i as f64 * 2.0).collect();
        let signed_up = AdxIndicator::new(14, Timescale::OneHour, "adx".into()).with_signed(true).compute(&market(&up)).unwrap();
        assert!(signed_up.score > 0.0);
    }
}
