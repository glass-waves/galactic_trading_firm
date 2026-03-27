use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position};
use types::market::MarketState;
use types::scoring::TimescaleScores;

use super::conditions::{all_conditions_met, parse_conditions, WindowCondition};

pub struct EntryRejectGateAction {
    name: String,
    conditions: Vec<WindowCondition>,
}

impl EntryRejectGateAction {
    pub fn new(name: String, conditions: Vec<WindowCondition>) -> Self {
        Self { name, conditions }
    }
}

impl Action for EntryRejectGateAction {
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
            ActionSignal::RejectEntry
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn entry_reject_gate_factory(config: &ActionConfig) -> Box<dyn Action> {
    let name = config
        .params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&config.instance_id)
        .to_string();

    let conditions = config
        .params
        .get("conditions")
        .and_then(|v| parse_conditions(v).ok())
        .unwrap_or_default();

    Box::new(EntryRejectGateAction::new(name, conditions))
}
