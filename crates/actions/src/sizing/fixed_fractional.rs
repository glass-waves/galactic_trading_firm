use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

/// sizes positions as a fixed fraction of available capital.
#[allow(dead_code)]
pub struct FixedFractionalSizing {
    fraction: f64,
    instance_id: String,
}

impl FixedFractionalSizing {
    pub fn new(fraction: f64, instance_id: String) -> Self {
        Self {
            fraction,
            instance_id,
        }
    }
}

impl Action for FixedFractionalSizing {
    fn name(&self) -> &str {
        "fixed_fractional"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Sizing
    }

    fn evaluate(
        &self,
        _position: Option<&Position>,
        _market: &MarketState,
        scores: &TimescaleScores,
    ) -> ActionSignal {
        // size based on fraction, direction from composite score
        let direction = if scores.composite >= 0.0 {
            TradeDirection::Long
        } else {
            TradeDirection::Short
        };

        ActionSignal::Enter {
            direction,
            size_fraction: self.fraction,
            reason: format!("fixed fractional sizing: {:.1}%", self.fraction * 100.0),
        }
    }
}

pub fn fixed_fractional_factory(config: &ActionConfig) -> Box<dyn Action> {
    let fraction = config
        .params
        .get("fraction")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.02);
    Box::new(FixedFractionalSizing::new(fraction, config.instance_id.clone()))
}
