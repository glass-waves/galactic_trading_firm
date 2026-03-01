use std::collections::HashMap;

use ta::indicators::RelativeStrengthIndex;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct StochasticRsiIndicator {
    rsi_period: usize,
    stoch_period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl StochasticRsiIndicator {
    pub fn new(rsi_period: usize, stoch_period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            rsi_period,
            stoch_period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for StochasticRsiIndicator {
    fn name(&self) -> &str {
        "stochastic_rsi"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.rsi_period + self.stoch_period + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        // compute RSI values
        let mut rsi = RelativeStrengthIndex::new(self.rsi_period).ok()?;
        let mut rsi_values = Vec::with_capacity(candles.len());
        for candle in candles {
            rsi_values.push(rsi.next(candle.close));
        }

        // apply stochastic formula on the last stoch_period RSI values
        let window_start = rsi_values.len().saturating_sub(self.stoch_period);
        let window = &rsi_values[window_start..];
        let max = window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = window.iter().cloned().fold(f64::INFINITY, f64::min);

        if (max - min).abs() < f64::EPSILON {
            return Some(IndicatorOutput {
                score: 0.0,
                raw_value: 50.0,
                metadata: HashMap::new(),
            });
        }

        let current_rsi = *rsi_values.last()?;
        let stoch_rsi = (current_rsi - min) / (max - min) * 100.0;
        let score = ((stoch_rsi - 50.0) / 50.0).clamp(-1.0, 1.0);

        Some(IndicatorOutput {
            score,
            raw_value: stoch_rsi,
            metadata: HashMap::new(),
        })
    }
}

pub fn stochastic_rsi_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let rsi_period = config.params.get("rsi_period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
    let stoch_period = config.params.get("stoch_period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
    Box::new(StochasticRsiIndicator::new(rsi_period, stoch_period, config.timescale, config.instance_id.clone()))
}
