use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

/// sizes position proportional to composite score strength.
/// higher scores → larger position; scores near threshold → minimum size.
#[allow(dead_code)]
pub struct ScoreScaledSizing {
    min_fraction: f64,
    max_fraction: f64,
    entry_threshold: f64,
    instance_id: String,
}

impl ScoreScaledSizing {
    pub fn new(min_fraction: f64, max_fraction: f64, entry_threshold: f64, instance_id: String) -> Self {
        Self {
            min_fraction,
            max_fraction,
            entry_threshold,
            instance_id,
        }
    }
}

impl Action for ScoreScaledSizing {
    fn name(&self) -> &str {
        "score_scaled"
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
        let score = scores.composite.abs();
        let range = 1.0 - self.entry_threshold;
        let fraction = if range > 0.0 {
            let t = ((score - self.entry_threshold) / range).clamp(0.0, 1.0);
            self.min_fraction + (self.max_fraction - self.min_fraction) * t
        } else {
            self.min_fraction
        };

        ActionSignal::Enter {
            direction: TradeDirection::Long,
            size_fraction: fraction,
            reason: format!("score_scaled: {:.4}", fraction),
        }
    }
}

pub fn score_scaled_factory(config: &ActionConfig) -> Box<dyn Action> {
    let min_fraction = config
        .params
        .get("min_fraction")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.02);
    let max_fraction = config
        .params
        .get("max_fraction")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.08);
    let entry_threshold = config
        .params
        .get("entry_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.58);
    Box::new(ScoreScaledSizing::new(
        min_fraction,
        max_fraction,
        entry_threshold,
        config.instance_id.clone(),
    ))
}
