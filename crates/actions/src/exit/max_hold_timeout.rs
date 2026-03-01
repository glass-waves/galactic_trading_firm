use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position};
use types::market::MarketState;
use types::scoring::TimescaleScores;

#[allow(dead_code)]
pub struct MaxHoldTimeout {
    max_hold_ms: i64,
    instance_id: String,
}

impl MaxHoldTimeout {
    pub fn new(max_hold_ms: i64, instance_id: String) -> Self {
        Self {
            max_hold_ms,
            instance_id,
        }
    }
}

impl Action for MaxHoldTimeout {
    fn name(&self) -> &str {
        "max_hold_timeout"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Exit
    }

    fn evaluate(
        &self,
        position: Option<&Position>,
        _market: &MarketState,
        _scores: &TimescaleScores,
    ) -> ActionSignal {
        let pos = match position {
            Some(p) => p,
            None => return ActionSignal::Hold,
        };

        if pos.hold_duration_ms >= self.max_hold_ms {
            ActionSignal::Exit {
                reason: ExitReason::MaxHoldTimeout,
            }
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn max_hold_timeout_factory(config: &ActionConfig) -> Box<dyn Action> {
    let ms = config
        .params
        .get("max_hold_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(3_600_000); // default 1 hour
    Box::new(MaxHoldTimeout::new(ms, config.instance_id.clone()))
}
