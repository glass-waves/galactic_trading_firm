use std::collections::HashMap;
use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// cross-ticker correlation indicator.
///
/// reads pre-computed cross_ticker_correlation from MarketState.
/// high correlation (>0.8) → -1.0 (diversification risk).
/// low correlation (<0.3) → +1.0 (independent opportunity).
/// returns None if correlation data is not available.
#[allow(dead_code)]
pub struct CrossCorrelationIndicator {
    high_threshold: f64,
    low_threshold: f64,
    timescale: Timescale,
    instance_id: String,
}

impl CrossCorrelationIndicator {
    pub fn new(
        high_threshold: f64,
        low_threshold: f64,
        timescale: Timescale,
        instance_id: String,
    ) -> Self {
        Self {
            high_threshold,
            low_threshold,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for CrossCorrelationIndicator {
    fn name(&self) -> &str {
        "cross_ticker_correlation"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let corr = market.cross_ticker_correlation?;

        // high correlation = bad (diversification risk) → negative
        // low correlation = good (independent opportunity) → positive
        let score = if corr >= self.high_threshold {
            -1.0
        } else if corr <= self.low_threshold {
            1.0
        } else {
            // linear interpolation between thresholds
            let range = self.high_threshold - self.low_threshold;
            if range > 0.0 {
                1.0 - 2.0 * (corr - self.low_threshold) / range
            } else {
                0.0
            }
        };

        let mut metadata = HashMap::new();
        metadata.insert("correlation".to_string(), corr);

        Some(IndicatorOutput {
            score,
            raw_value: corr,
            metadata,
        })
    }
}

pub fn cross_correlation_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let high = config
        .params
        .get("high_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.8);
    let low = config
        .params
        .get("low_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.3);
    Box::new(CrossCorrelationIndicator::new(
        high,
        low,
        config.timescale,
        config.instance_id.clone(),
    ))
}
