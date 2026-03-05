use std::collections::HashMap;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// momentum persistence indicator (ROC of ROC — second derivative of price).
///
/// computes rate of change, then rate of change of that.
/// accelerating momentum → positive, decelerating → negative.
#[allow(dead_code)]
pub struct MomentumPersistenceIndicator {
    roc_period: usize,
    timescale: Timescale,
    instance_id: String,
}

impl MomentumPersistenceIndicator {
    pub fn new(roc_period: usize, timescale: Timescale, instance_id: String) -> Self {
        Self {
            roc_period,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for MomentumPersistenceIndicator {
    fn name(&self) -> &str {
        "momentum_persistence"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.roc_period * 2 + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        let min_len = self.roc_period * 2 + 1;
        if candles.len() < min_len {
            return None;
        }

        // compute ROC series for enough points to get a second ROC
        let n = candles.len();
        let mut roc_values = Vec::new();
        for i in self.roc_period..n {
            let prev = candles[i - self.roc_period].close;
            if prev <= 0.0 {
                return None;
            }
            roc_values.push((candles[i].close - prev) / prev);
        }

        if roc_values.len() < self.roc_period + 1 {
            return None;
        }

        // ROC of ROC (second derivative)
        let roc_len = roc_values.len();
        let current_roc = roc_values[roc_len - 1];
        let prev_roc = roc_values[roc_len - 1 - self.roc_period];

        let roc_of_roc = current_roc - prev_roc;

        // normalize: accelerating → +1.0, decelerating → -0.5
        // use a normalization factor based on typical ROC magnitude
        let score = if roc_of_roc > 0.0 {
            (roc_of_roc / 0.02).min(1.0)
        } else {
            (roc_of_roc / 0.02).max(-1.0) * 0.5
        };

        let mut metadata = HashMap::new();
        metadata.insert("roc_of_roc".to_string(), roc_of_roc);
        metadata.insert("current_roc".to_string(), current_roc);

        Some(IndicatorOutput {
            score,
            raw_value: roc_of_roc,
            metadata,
        })
    }
}

pub fn momentum_persistence_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let period = config
        .params
        .get("period")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;
    Box::new(MomentumPersistenceIndicator::new(
        period,
        config.timescale,
        config.instance_id.clone(),
    ))
}
