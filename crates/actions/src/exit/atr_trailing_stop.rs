use ta::indicators::AverageTrueRange;
use ta::Next;

use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position, TradeDirection};
use types::market::{MarketState, Timescale};
use types::scoring::TimescaleScores;

#[allow(dead_code)]
pub struct AtrTrailingStop {
    atr_period: usize,
    multiplier: f64,
    timescale: Timescale,
    instance_id: String,
}

impl AtrTrailingStop {
    pub fn new(atr_period: usize, multiplier: f64, timescale: Timescale, instance_id: String) -> Self {
        Self {
            atr_period,
            multiplier,
            timescale,
            instance_id,
        }
    }

    fn compute_atr(&self, market: &MarketState) -> Option<f64> {
        let candles = market.candles.get(&self.timescale)?;
        if candles.len() < self.atr_period + 1 {
            return None;
        }
        let mut atr = AverageTrueRange::new(self.atr_period).ok()?;
        let mut last = 0.0;
        for candle in candles {
            let item: ta::DataItem = candle.into();
            last = atr.next(&item);
        }
        Some(last)
    }
}

impl Action for AtrTrailingStop {
    fn name(&self) -> &str {
        "atr_trailing_stop"
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
        let pos = match position {
            Some(p) => p,
            None => return ActionSignal::Hold,
        };

        let atr = match self.compute_atr(market) {
            Some(v) => v,
            None => return ActionSignal::Hold,
        };

        let stop_distance = atr * self.multiplier;

        match pos.direction {
            TradeDirection::Long => {
                let stop_level = pos.high_water_mark - stop_distance;
                if market.last_price <= stop_level {
                    ActionSignal::Exit {
                        reason: ExitReason::TrailingStop,
                    }
                } else {
                    ActionSignal::Hold
                }
            }
            TradeDirection::Short => {
                let stop_level = pos.low_water_mark + stop_distance;
                if market.last_price >= stop_level {
                    ActionSignal::Exit {
                        reason: ExitReason::TrailingStop,
                    }
                } else {
                    ActionSignal::Hold
                }
            }
        }
    }
}

pub fn atr_trailing_stop_factory(config: &ActionConfig) -> Box<dyn Action> {
    let period = config.params.get("atr_period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
    let mult = config.params.get("multiplier").and_then(|v| v.as_f64()).unwrap_or(2.0);
    let ts_str = config.params.get("timescale").and_then(|v| v.as_str()).unwrap_or("FiveMinute");
    let timescale = match ts_str {
        "OneMinute" => Timescale::OneMinute,
        "OneHour" => Timescale::OneHour,
        _ => Timescale::FiveMinute,
    };
    Box::new(AtrTrailingStop::new(period, mult, timescale, config.instance_id.clone()))
}
