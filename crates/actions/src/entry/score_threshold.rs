use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

#[allow(dead_code)]
pub struct ScoreThresholdEntry {
    entry_threshold: f64,
    short_threshold: f64,
    instance_id: String,
}

impl ScoreThresholdEntry {
    pub fn new(entry_threshold: f64, short_threshold: f64, instance_id: String) -> Self {
        Self {
            entry_threshold,
            short_threshold,
            instance_id,
        }
    }
}

impl Action for ScoreThresholdEntry {
    fn name(&self) -> &str {
        "score_threshold_entry"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Entry
    }

    fn evaluate(
        &self,
        position: Option<&Position>,
        _market: &MarketState,
        scores: &TimescaleScores,
    ) -> ActionSignal {
        // don't enter if already in a position
        if position.is_some() {
            return ActionSignal::Hold;
        }

        if scores.composite >= self.entry_threshold {
            ActionSignal::Enter {
                direction: TradeDirection::Long,
                size_fraction: 1.0,
                reason: format!("composite {:.3} >= {:.3}", scores.composite, self.entry_threshold),
            }
        } else if scores.composite <= self.short_threshold {
            ActionSignal::Enter {
                direction: TradeDirection::Short,
                size_fraction: 1.0,
                reason: format!("composite {:.3} <= {:.3}", scores.composite, self.short_threshold),
            }
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn score_threshold_entry_factory(config: &ActionConfig) -> Box<dyn Action> {
    let entry = config
        .params
        .get("entry_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.65);
    let short = config
        .params
        .get("short_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(-0.65);
    Box::new(ScoreThresholdEntry::new(entry, short, config.instance_id.clone()))
}
