use std::collections::HashMap;

use ta::indicators::MovingAverageConvergenceDivergence;
use ta::Next;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct MacdIndicator {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    timescale: Timescale,
    instance_id: String,
    normalization_factor: f64,
    /// when above 0, the histogram is scored against `close × normalization_pct` (a 1 × score = a histogram
    /// of that fraction of price), which makes the score invariant to the price level; the default
    /// 0 keeps `normalization_factor` ($1 for every promoted config so far — a hidden price-level
    /// parameter, docs/research_queue.md RQ-5/RQ-8).
    normalization_pct: f64,
}

impl MacdIndicator {
    pub fn new(
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        timescale: Timescale,
        instance_id: String,
    ) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
            timescale,
            instance_id,
            normalization_factor: 1.0,
            normalization_pct: 0.0,
        }
    }

    pub fn with_normalization_pct(mut self, pct: f64) -> Self {
        self.normalization_pct = pct;
        self
    }
}

impl Indicator for MacdIndicator {
    fn name(&self) -> &str {
        "macd"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.slow_period + self.signal_period
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.min_lookback() {
            return None;
        }

        let mut macd = MovingAverageConvergenceDivergence::new(
            self.fast_period,
            self.slow_period,
            self.signal_period,
        )
        .ok()?;

        let mut last_output = None;
        for candle in candles {
            last_output = Some(macd.next(candle.close));
        }

        let output = last_output?;
        let histogram = output.histogram;
        let avg_price = candles.last()?.close;
        let norm = if self.normalization_pct > 0.0 {
            avg_price * self.normalization_pct
        } else if self.normalization_factor > 0.0 {
            self.normalization_factor
        } else {
            avg_price * 0.01
        };
        let score = (histogram / norm).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("macd_line".to_string(), output.macd);
        metadata.insert("signal_line".to_string(), output.signal);
        metadata.insert("histogram".to_string(), output.histogram);

        Some(IndicatorOutput {
            score,
            raw_value: histogram,
            metadata,
        })
    }
}

pub fn macd_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let fast = config
        .params
        .get("fast_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(12) as usize;
    let slow = config
        .params
        .get("slow_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(26) as usize;
    let signal = config
        .params
        .get("signal_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(9) as usize;
    let norm = config
        .params
        .get("normalization_factor")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let pct = config.params.get("normalization_pct").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let mut ind = MacdIndicator::new(fast, slow, signal, config.timescale, config.instance_id.clone()).with_normalization_pct(pct);
    ind.normalization_factor = norm;
    Box::new(ind)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn market(closes: &[f64]) -> MarketState {
        types::test_fixtures::make_market_state(Timescale::FiveMinute, closes)
    }

    #[test]
    fn pct_normalisation_is_invariant_to_the_price_level_and_dollar_is_not() {
        let base: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64 * 0.3).sin() * 1.5 + i as f64 * 0.05).collect();
        let x10: Vec<f64> = base.iter().map(|c| c * 10.0).collect();
        let dollar = |m: &MarketState| MacdIndicator::new(12, 26, 9, Timescale::FiveMinute, "m".into()).compute(m).unwrap().score;
        let pct = |m: &MarketState| MacdIndicator::new(12, 26, 9, Timescale::FiveMinute, "m".into()).with_normalization_pct(0.005).compute(m).unwrap().score;
        assert!((pct(&market(&base)) - pct(&market(&x10))).abs() < 1e-9);
        assert!((dollar(&market(&base)) - dollar(&market(&x10))).abs() > 1e-3, "the $1 norm scores a 10x price differently");
    }
}
