use std::collections::HashMap;

use types::indicator::{Indicator, IndicatorConfig, IndicatorOutput};
use types::market::{MarketState, Timescale};

#[allow(dead_code)]
pub struct VwapDistanceIndicator {
    timescale: Timescale,
    instance_id: String,
    scale_factor: f64,
}

impl VwapDistanceIndicator {
    pub fn new(timescale: Timescale, instance_id: String) -> Self {
        Self {
            timescale,
            instance_id,
            scale_factor: 0.005,
        }
    }
}

impl Indicator for VwapDistanceIndicator {
    fn name(&self) -> &str {
        "vwap_distance"
    }

    fn timescale(&self) -> Timescale {
        self.timescale
    }

    fn min_lookback(&self) -> usize {
        1
    }

    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput> {
        if market.session_vwap.abs() < f64::EPSILON {
            return None;
        }

        let pct_distance = (market.last_price - market.session_vwap) / market.session_vwap;
        let score = (pct_distance / self.scale_factor).clamp(-1.0, 1.0);

        let mut metadata = HashMap::new();
        metadata.insert("vwap".to_string(), market.session_vwap);
        metadata.insert("pct_distance".to_string(), pct_distance);

        Some(IndicatorOutput {
            score,
            raw_value: pct_distance,
            metadata,
        })
    }
}

pub fn vwap_distance_factory(config: &IndicatorConfig) -> Box<dyn Indicator> {
    Box::new(VwapDistanceIndicator::new(config.timescale, config.instance_id.clone()))
}
