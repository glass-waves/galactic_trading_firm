use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

use super::conditions::{all_conditions_met, failing_conditions, parse_conditions, WindowCondition};

pub struct EntryWindowAction {
    name: String,
    conditions: Vec<WindowCondition>,
    direction: TradeDirection,
}

impl EntryWindowAction {
    pub fn new(name: String, conditions: Vec<WindowCondition>, direction: TradeDirection) -> Self {
        Self {
            name,
            conditions,
            direction,
        }
    }
}

impl Action for EntryWindowAction {
    fn name(&self) -> &str {
        &self.name
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
        if position.is_some() {
            return ActionSignal::Hold;
        }

        let indicator_scores = scores.indicator_scores.as_ref();

        if all_conditions_met(&self.conditions, scores, indicator_scores) {
            ActionSignal::Enter {
                direction: self.direction,
                size_fraction: 1.0,
                reason: format!("window:{}", self.name),
            }
        } else {
            ActionSignal::Hold
        }
    }

    /// report failing conditions, but only when the window's composite floor
    /// (if any) is already met — i.e. the window was "close". windows with no
    /// composite floor always report.
    fn diagnose(&self, _market: &MarketState, scores: &TimescaleScores) -> Option<String> {
        let indicator_scores = scores.indicator_scores.as_ref();
        let composite_ok = self.conditions.iter().all(|c| match c {
            WindowCondition::CompositeMin { .. } | WindowCondition::CompositeMax { .. } => {
                c.evaluate(scores, indicator_scores)
            }
            _ => true,
        });
        if !composite_ok {
            return None;
        }
        let failing = failing_conditions(&self.conditions, scores, indicator_scores);
        if failing.is_empty() {
            None
        } else {
            Some(format!("{}: {}", self.name, failing.join(", ")))
        }
    }
}

pub fn entry_window_factory(config: &ActionConfig) -> Box<dyn Action> {
    let name = config
        .params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&config.instance_id)
        .to_string();

    let direction = config
        .params
        .get("direction")
        .and_then(|v| v.as_str())
        .map(|d| match d {
            "short" | "Short" => TradeDirection::Short,
            _ => TradeDirection::Long,
        })
        .unwrap_or(TradeDirection::Long);

    let conditions = config
        .params
        .get("conditions")
        .and_then(|v| parse_conditions(v).ok())
        .unwrap_or_default();

    Box::new(EntryWindowAction::new(name, conditions, direction))
}
