use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

use super::conditions::{all_conditions_met, parse_conditions, WindowCondition};

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
