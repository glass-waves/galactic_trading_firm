use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

/// relative volume indicator.
///
/// RVOL = current_volume / average_volume over lookback.
/// high volume (above threshold) → positive score, low volume → negative score.
#[allow(dead_code)]
pub struct RvolIndicator {
    lookback_period: usize,
    high_threshold: f64,
    low_threshold: f64,
    timescale: Timescale,
    instance_id: String,
}

impl RvolIndicator {
    pub fn new(
        lookback_period: usize,
        high_threshold: f64,
        low_threshold: f64,
        timescale: Timescale,
        instance_id: String,
    ) -> Self {
        Self {
            lookback_period,
            high_threshold,
            low_threshold,
            timescale,
            instance_id,
        }
    }
}

impl Indicator for RvolIndicator {
    fn name(&self) -> &str {
        "relative_volume"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        self.lookback_period + 1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.lookback_period + 1 {
            return None;
        }

        let current_vol = candles.last()?.volume;

        // average volume over lookback (excluding current candle)
        let start = candles.len() - 1 - self.lookback_period;
        let end = candles.len() - 1;
        let avg_vol: f64 = candles[start..end].iter().map(|c| c.volume).sum::<f64>()
            / self.lookback_period as f64;

        if avg_vol <= 0.0 {
            return None;
        }

        let rvol = current_vol / avg_vol;

        // score: high vol → +1.0, low vol → -0.3, normal → 0.0
        let score = if rvol >= self.high_threshold {
            ((rvol - self.high_threshold) / self.high_threshold).min(1.0)
        } else if rvol <= self.low_threshold {
            -0.3 * (1.0 - rvol / self.low_threshold).min(1.0)
        } else {
            0.0
        };

        let mut metadata = std::collections::HashMap::new();
        metadata.insert("rvol".to_string(), rvol);

        Some(IndicatorOutput {
            score,
            raw_value: rvol,
            metadata,
        })
    }
}

pub fn rvol_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    let lookback = config
        .params
        .get("lookback_period")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let high_threshold = config
        .params
        .get("high_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.5);
    let low_threshold = config
        .params
        .get("low_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    Box::new(RvolIndicator::new(
        lookback,
        high_threshold,
        low_threshold,
        config.timescale,
        config.instance_id.clone(),
    ))
}
