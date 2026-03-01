use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

/// moves stop to breakeven after a configurable profit trigger is reached.
#[allow(dead_code)]
pub struct BreakevenStop {
    trigger_pct: f64,
    instance_id: String,
}

impl BreakevenStop {
    pub fn new(trigger_pct: f64, instance_id: String) -> Self {
        Self {
            trigger_pct,
            instance_id,
        }
    }
}

impl Action for BreakevenStop {
    fn name(&self) -> &str {
        "breakeven_stop"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Monitor
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

        // check if profit has exceeded trigger
        let profit_pct = match pos.direction {
            TradeDirection::Long => {
                (pos.high_water_mark - pos.entry_price) / pos.entry_price
            }
            TradeDirection::Short => {
                (pos.entry_price - pos.low_water_mark) / pos.entry_price
            }
        };

        if profit_pct >= self.trigger_pct {
            ActionSignal::ModifyStop {
                new_stop_price: pos.entry_price,
            }
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn breakeven_stop_factory(config: &ActionConfig) -> Box<dyn Action> {
    let trigger = config
        .params
        .get("trigger_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.01);
    Box::new(BreakevenStop::new(trigger, config.instance_id.clone()))
}
