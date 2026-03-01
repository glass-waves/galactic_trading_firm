use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position};
use types::market::MarketState;
use types::scoring::TimescaleScores;

#[allow(dead_code)]
pub struct SessionCloseExit {
    force_exit_by: String, // "HH:MM" format
    instance_id: String,
}

impl SessionCloseExit {
    pub fn new(force_exit_by: String, instance_id: String) -> Self {
        Self {
            force_exit_by,
            instance_id,
        }
    }

    fn parse_hm(&self) -> Option<(u32, u32)> {
        let parts: Vec<&str> = self.force_exit_by.split(':').collect();
        if parts.len() == 2 {
            let h = parts[0].parse().ok()?;
            let m = parts[1].parse().ok()?;
            Some((h, m))
        } else {
            None
        }
    }
}

impl Action for SessionCloseExit {
    fn name(&self) -> &str {
        "session_close"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Exit
    }

    fn evaluate(
        &self,
        position: Option<&Position>,
        market: &MarketState,
        _scores: &TimescaleScores,
    ) -> ActionSignal {
        if position.is_none() {
            return ActionSignal::Hold;
        }

        if let Some((exit_hour, exit_min)) = self.parse_hm() {
            let ts = market.timestamp;
            let current_minutes = ts.format("%H").to_string().parse::<u32>().unwrap_or(0) * 60
                + ts.format("%M").to_string().parse::<u32>().unwrap_or(0);
            let exit_minutes = exit_hour * 60 + exit_min;

            if current_minutes >= exit_minutes {
                return ActionSignal::Exit {
                    reason: ExitReason::SessionClose,
                };
            }
        }

        ActionSignal::Hold
    }
}

pub fn session_close_factory(config: &ActionConfig) -> Box<dyn Action> {
    let time = config
        .params
        .get("force_exit_by")
        .and_then(|v| v.as_str())
        .unwrap_or("15:55")
        .to_string();
    Box::new(SessionCloseExit::new(time, config.instance_id.clone()))
}
