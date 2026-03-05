use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position};
use types::market::MarketState;
use types::scoring::TimescaleScores;

#[allow(dead_code)]
pub struct MaxHoldTimeout {
    max_hold_ms: i64,
    /// additional ms to hold when position is profitable.
    profit_extension_ms: i64,
    /// ms to subtract from hold when position is losing (floor at 0).
    loss_reduction_ms: i64,
    instance_id: String,
}

impl MaxHoldTimeout {
    pub fn new(
        max_hold_ms: i64,
        profit_extension_ms: i64,
        loss_reduction_ms: i64,
        instance_id: String,
    ) -> Self {
        Self {
            max_hold_ms,
            profit_extension_ms,
            loss_reduction_ms,
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

        // adaptive hold: extend for profitable, reduce for losing
        let effective_hold = if pos.unrealized_pnl > 0.0 {
            self.max_hold_ms + self.profit_extension_ms
        } else if pos.unrealized_pnl < 0.0 {
            (self.max_hold_ms - self.loss_reduction_ms).max(0)
        } else {
            self.max_hold_ms
        };

        if pos.hold_duration_ms >= effective_hold {
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
    let profit_ext = config
        .params
        .get("profit_extension_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let loss_red = config
        .params
        .get("loss_reduction_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    Box::new(MaxHoldTimeout::new(ms, profit_ext, loss_red, config.instance_id.clone()))
}
