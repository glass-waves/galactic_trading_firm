use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

#[allow(dead_code)]
pub struct FixedPctStop {
    stop_loss_pct: f64,
    instance_id: String,
}

impl FixedPctStop {
    pub fn new(stop_loss_pct: f64, instance_id: String) -> Self {
        Self {
            stop_loss_pct,
            instance_id,
        }
    }
}

impl Action for FixedPctStop {
    fn name(&self) -> &str {
        "fixed_pct_stop"
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

        let loss_pct = match pos.direction {
            TradeDirection::Long => (pos.entry_price - pos.current_price) / pos.entry_price,
            TradeDirection::Short => (pos.current_price - pos.entry_price) / pos.entry_price,
        };

        if loss_pct >= self.stop_loss_pct {
            ActionSignal::Exit {
                reason: ExitReason::HardStop,
            }
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn fixed_pct_stop_factory(config: &ActionConfig) -> Box<dyn Action> {
    let pct = config
        .params
        .get("stop_loss_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.02);
    Box::new(FixedPctStop::new(pct, config.instance_id.clone()))
}
