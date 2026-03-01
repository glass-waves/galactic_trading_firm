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
}

impl AdxIndicator {
    pub fn new(period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            period,
            timescale,
            instance_id,
        }
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
        let score = (last_adx / 50.0 - 1.0).clamp(-1.0, 1.0);

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
    Box::new(AdxIndicator::new(period, config.timescale, config.instance_id.clone()))
}
