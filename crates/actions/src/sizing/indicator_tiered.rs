//! tiered sizing keyed on an indicator value (research 2026-09-24, proposal "tiered sizing").
//!
//! reads `scores.indicator_scores[instance_id]` (an instance id or `{id}.{meta}` key such as
//! `vpin_1m.raw_vpin`) and returns `base_fraction * mult` for the highest tier whose `min` the
//! value reaches. below every tier the fraction is `base_fraction * fallback_mult` (default 0,
//! which makes the tick loop skip the entry). params:
//! `{"instance_id": "vpin_1m.raw_vpin", "base_fraction": 0.36,
//!   "tiers": [{"min": 0.217, "mult": 1.0}, {"min": 0.15, "mult": 0.5}], "fallback_mult": 0.0}`
use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

#[derive(Clone, Debug)]
pub struct Tier {
    pub min: f64,
    pub mult: f64,
}

pub struct IndicatorTieredSizing {
    instance_id: String,
    base_fraction: f64,
    /// sorted by `min` descending
    tiers: Vec<Tier>,
    fallback_mult: f64,
}

impl IndicatorTieredSizing {
    pub fn new(instance_id: String, base_fraction: f64, mut tiers: Vec<Tier>, fallback_mult: f64) -> Self {
        tiers.sort_by(|a, b| b.min.partial_cmp(&a.min).unwrap_or(std::cmp::Ordering::Equal));
        Self { instance_id, base_fraction, tiers, fallback_mult }
    }

    pub fn multiplier_for(&self, value: Option<f64>) -> f64 {
        match value {
            Some(v) => self
                .tiers
                .iter()
                .find(|t| v >= t.min)
                .map(|t| t.mult)
                .unwrap_or(self.fallback_mult),
            None => self.fallback_mult,
        }
    }
}

impl Action for IndicatorTieredSizing {
    fn name(&self) -> &str {
        "indicator_tiered"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Sizing
    }

    fn evaluate(&self, _position: Option<&Position>, _market: &MarketState, scores: &TimescaleScores) -> ActionSignal {
        let value = scores
            .indicator_scores
            .as_ref()
            .and_then(|m| m.get(&self.instance_id).copied())
            .flatten();
        let mult = self.multiplier_for(value);
        let direction = if scores.composite >= 0.0 { TradeDirection::Long } else { TradeDirection::Short };
        ActionSignal::Enter {
            direction,
            size_fraction: self.base_fraction * mult,
            reason: format!("indicator_tiered {}={:?} x{:.2}", self.instance_id, value, mult),
        }
    }
}

pub fn indicator_tiered_factory(config: &ActionConfig) -> Box<dyn Action> {
    let instance_id = config
        .params
        .get("instance_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base_fraction = config.params.get("base_fraction").and_then(|v| v.as_f64()).unwrap_or(0.3);
    let fallback_mult = config.params.get("fallback_mult").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let tiers = config
        .params
        .get("tiers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    Some(Tier {
                        min: t.get("min")?.as_f64()?,
                        mult: t.get("mult")?.as_f64()?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Box::new(IndicatorTieredSizing::new(instance_id, base_fraction, tiers, fallback_mult))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_highest_matching_tier_and_falls_back() {
        let s = IndicatorTieredSizing::new(
            "v".into(),
            0.36,
            vec![Tier { min: 0.15, mult: 0.5 }, Tier { min: 0.217, mult: 1.0 }],
            0.0,
        );
        assert_eq!(s.multiplier_for(Some(0.30)), 1.0);
        assert_eq!(s.multiplier_for(Some(0.217)), 1.0);
        assert_eq!(s.multiplier_for(Some(0.18)), 0.5);
        assert_eq!(s.multiplier_for(Some(0.10)), 0.0);
        assert_eq!(s.multiplier_for(None), 0.0);
    }
}
