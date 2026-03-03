use ta::indicators::AverageTrueRange;
use ta::Next;

use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, TradeDirection};
use types::market::{MarketState, Timescale};
use types::scoring::TimescaleScores;

/// position sizes inversely scaled by realized volatility (ATR).
///
/// formula: adjusted_fraction = base_fraction / (current_atr / baseline_atr)
/// clamped to [min_fraction, max_fraction].
#[allow(dead_code)]
pub struct VolatilityScaledSizing {
    base_fraction: f64,
    atr_period: usize,
    baseline_atr: f64,
    max_fraction: f64,
    min_fraction: f64,
    timescale: Timescale,
    instance_id: String,
}

impl VolatilityScaledSizing {
    pub fn new(
        base_fraction: f64,
        atr_period: usize,
        baseline_atr: f64,
        max_fraction: f64,
        min_fraction: f64,
        timescale: Timescale,
        instance_id: String,
    ) -> Self {
        Self {
            base_fraction,
            atr_period,
            baseline_atr,
            max_fraction,
            min_fraction,
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

impl Action for VolatilityScaledSizing {
    fn name(&self) -> &str {
        "volatility_scaled"
    }

    fn phase(&self) -> ActionPhase {
        ActionPhase::Sizing
    }

    fn evaluate(
        &self,
        position: Option<&types::action::Position>,
        market: &MarketState,
        scores: &TimescaleScores,
    ) -> ActionSignal {
        // if already in a position, hold
        if position.is_some() {
            return ActionSignal::Hold;
        }

        let direction = if scores.composite >= 0.0 {
            TradeDirection::Long
        } else {
            TradeDirection::Short
        };

        let fraction = match self.compute_atr(market) {
            Some(current_atr) if current_atr > f64::EPSILON => {
                let ratio = current_atr / self.baseline_atr;
                (self.base_fraction / ratio).clamp(self.min_fraction, self.max_fraction)
            }
            _ => self.base_fraction,
        };

        ActionSignal::Enter {
            direction,
            size_fraction: fraction,
            reason: format!("volatility-scaled sizing: {:.2}%", fraction * 100.0),
        }
    }
}

pub fn volatility_scaled_factory(config: &ActionConfig) -> Box<dyn Action> {
    let base_fraction = config.params.get("base_fraction").and_then(|v| v.as_f64()).unwrap_or(0.01);
    let atr_period = config.params.get("atr_period").and_then(|v| v.as_u64()).unwrap_or(14) as usize;
    let baseline_atr = config.params.get("baseline_atr").and_then(|v| v.as_f64()).unwrap_or(2.0);
    let max_fraction = config.params.get("max_fraction").and_then(|v| v.as_f64()).unwrap_or(0.03);
    let min_fraction = config.params.get("min_fraction").and_then(|v| v.as_f64()).unwrap_or(0.002);
    let ts_str = config.params.get("timescale").and_then(|v| v.as_str()).unwrap_or("FiveMinute");
    let timescale = match ts_str {
        "OneMinute" => Timescale::OneMinute,
        "OneHour" => Timescale::OneHour,
        _ => Timescale::FiveMinute,
    };
    Box::new(VolatilityScaledSizing::new(
        base_fraction,
        atr_period,
        baseline_atr,
        max_fraction,
        min_fraction,
        timescale,
        config.instance_id.clone(),
    ))
}
