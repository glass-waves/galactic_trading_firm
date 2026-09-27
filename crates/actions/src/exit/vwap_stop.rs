use types::action::{Action, ActionConfig, ActionPhase, ActionSignal, ExitReason, Position, TradeDirection};
use types::market::MarketState;
use types::scoring::TimescaleScores;

/// session-VWAP stop: a short exits once the bar closes more than `buffer_pct` percent
/// above the session VWAP (the market has reclaimed VWAP), a long once it closes more than
/// `buffer_pct` below it. meant for VWAP-anchored entries, where VWAP is the thesis level.
///
/// exit actions apply to every open position, so the stop can be scoped to the positions
/// of one entry window: with `scope_min_max_hold_ms` > 0 it only acts while the open
/// position's active max hold (`MarketState.position_context.max_hold_ms`, which the engine
/// sets from the entering window's `exit_overrides.max_hold_ms` and resets on exit) is at
/// least that long. a window that carries a long max-hold override therefore gets the stop,
/// the ordinary windows (engine default max hold) do not. 0 = every position.
///
/// the exit is reported as `ExitReason::FilterAlignment` ("the level the entry was aligned
/// to no longer holds") so it is distinguishable from the hard / trailing stops in the
/// trade log.
#[allow(dead_code)]
pub struct VwapStop {
    buffer_pct: f64,
    scope_min_max_hold_ms: i64,
    instance_id: String,
}

impl VwapStop {
    pub fn new(buffer_pct: f64, scope_min_max_hold_ms: i64, instance_id: String) -> Self {
        Self { buffer_pct, scope_min_max_hold_ms, instance_id }
    }

    fn in_scope(&self, market: &MarketState) -> bool {
        if self.scope_min_max_hold_ms <= 0 {
            return true;
        }
        market
            .position_context
            .as_ref()
            .is_some_and(|pc| pc.max_hold_ms >= self.scope_min_max_hold_ms)
    }
}

impl Action for VwapStop {
    fn name(&self) -> &str {
        "vwap_stop"
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
        let Some(pos) = position else { return ActionSignal::Hold };
        let vwap = market.session_vwap;
        if !(vwap.is_finite() && vwap > 0.0 && market.last_price.is_finite() && self.in_scope(market)) {
            return ActionSignal::Hold;
        }
        let buf = self.buffer_pct / 100.0;
        let hit = match pos.direction {
            TradeDirection::Short => market.last_price > vwap * (1.0 + buf),
            TradeDirection::Long => market.last_price < vwap * (1.0 - buf),
        };
        if hit {
            ActionSignal::Exit { reason: ExitReason::FilterAlignment }
        } else {
            ActionSignal::Hold
        }
    }
}

pub fn vwap_stop_factory(config: &ActionConfig) -> Box<dyn Action> {
    let buffer_pct = config.params.get("buffer_pct").and_then(|v| v.as_f64()).unwrap_or(0.4);
    let scope = config
        .params
        .get("scope_min_max_hold_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    Box::new(VwapStop::new(buffer_pct, scope, config.instance_id.clone()))
}
