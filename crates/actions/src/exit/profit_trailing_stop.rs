use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

/// trails a stop based on a fraction of peak unrealized profit.
///
/// for a long position:
///   peak_profit_pct = (high_water_mark - entry_price) / entry_price
///   stop_price = entry_price * (1 + peak_profit_pct * (1 - giveback_fraction))
///
/// the stop ratchets up as the high water mark advances. small winners get tight
/// stops (protecting thin profit). large winners get wide stops (giving room to run).
/// only activates once peak profit exceeds `min_profit_pct`.
#[allow(dead_code)]
pub struct ProfitTrailingStop {
    /// max fraction of peak profit to give back. 0.50 = keep at least 50% of peak.
    giveback_fraction: f64,
    /// minimum peak profit % before the stop activates.
    min_profit_pct: f64,
    instance_id: String,
}

impl ProfitTrailingStop {
    pub fn new(giveback_fraction: f64, min_profit_pct: f64, instance_id: String) -> Self {
        Self {
            giveback_fraction: giveback_fraction.clamp(0.0, 1.0),
            min_profit_pct: min_profit_pct.max(0.0),
            instance_id,
        }
    }

    fn compute_stop(&self, pos: &Position) -> Option<f64> {
        let (peak_profit_pct, stop_price) = match pos.direction {
            TradeDirection::Long => {
                let peak = (pos.high_water_mark - pos.entry_price) / pos.entry_price;
                if peak < self.min_profit_pct {
                    return None;
                }
                let keep_pct = peak * (1.0 - self.giveback_fraction);
                (peak, pos.entry_price * (1.0 + keep_pct))
            }
            TradeDirection::Short => {
                let peak = (pos.entry_price - pos.low_water_mark) / pos.entry_price;
                if peak < self.min_profit_pct {
                    return None;
                }
                let keep_pct = peak * (1.0 - self.giveback_fraction);
                (peak, pos.entry_price * (1.0 - keep_pct))
            }
        };

        if peak_profit_pct >= self.min_profit_pct {
            Some(stop_price)
        } else {
            None
        }
    }
}

impl Action for ProfitTrailingStop {
    fn name(&self) -> &str {
        "profit_trailing_stop"
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

        let stop_price = match self.compute_stop(pos) {
            Some(s) => s,
            None => return ActionSignal::Hold,
        };

        let triggered = match pos.direction {
            TradeDirection::Long => pos.current_price <= stop_price,
            TradeDirection::Short => pos.current_price >= stop_price,
        };

        if triggered {
            ActionSignal::Exit {
                reason: ExitReason::TrailingStop,
            }
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn profit_trailing_stop_factory(config: &ActionConfig) -> Box<dyn Action> {
    let giveback = config
        .params
        .get("giveback_fraction")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.50);
    let min_profit = config
        .params
        .get("min_profit_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0005); // 0.05%
    Box::new(ProfitTrailingStop::new(giveback, min_profit, config.instance_id.clone()))
}
