use std::collections::HashMap;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// market breadth indicator.
///
/// compares the stock's rolling return vs the index return.
/// outperformance → positive score, underperformance → negative.
/// returns None if index_return is not available.
#[allow(dead_code)]
pub struct MarketBreadthIndicator {
    lookback_period: usize,
    normalization: f64,
    timescale: Timescale,
    instance_id: String,
}

impl MarketBreadthIndicator {
    pub fn new(
        lookback_period: usize,
        normalization: f64,
        timescale: Timescale,
        instance_id: String,
    ) -> Self {
        Self {
            lookback_period,
            normalization,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for MarketBreadthIndicator {
    fn name(&self) -> &str {
        "market_breadth"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.lookback_period + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let index_return = market.index_return?;

        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.lookback_period + 1 {
            return None;
        }

        let current = candles.last()?.close;
        let past = candles[candles.len() - 1 - self.lookback_period].close;
        if past <= 0.0 {
            return None;
        }

        let stock_return = (current - past) / past;
        let relative = stock_return - index_return;

        // normalize to [-1.0, 1.0]
        let score = (relative / self.normalization).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("stock_return".to_string(), stock_return);
        metadata.insert("index_return".to_string(), index_return);
        metadata.insert("relative".to_string(), relative);

        Some(IndicatorOutput {
            score,
            raw_value: relative,
            metadata,
        })
    }
}

pub fn market_breadth_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let lookback = config
        .params
        .get("lookback_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let normalization = config
        .params
        .get("normalization")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.02);
    Box::new(MarketBreadthIndicator::new(
        lookback,
        normalization,
        config.timescale,
        config.instance_id.clone(),
    ))
}
