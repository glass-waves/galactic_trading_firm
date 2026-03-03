use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use tracing::warn;
use types::action::{Action, ActionSignal};
use types::indicator::{Indicator, IndicatorConfig};
use types::market::MarketState;
use types::scoring::ScoringConfig;
use types::tick_result::{TickEvent, TickResult};

use crate::position::{PositionManager, TradeRecord};
use crate::scoring::compute_composite_with_indicators;
use indicators::aggregation::compute_timescale_scores;

/// the main trading engine that orchestrates the tick loop.
pub struct TradingEngine {
    indicators: HashMap<String, Box<dyn Indicator>>,
    indicator_configs: Vec<IndicatorConfig>,
    scoring_config: ScoringConfig,
    entry_actions: Vec<Box<dyn Action>>,
    monitor_actions: Vec<Box<dyn Action>>,
    exit_actions: Vec<Box<dyn Action>>,
    sizing_actions: Vec<Box<dyn Action>>,
    position_manager: PositionManager,
    ticker: String,
    capital: f64,
    completed_trades: Vec<TradeRecord>,
}

impl TradingEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        indicators: HashMap<String, Box<dyn Indicator>>,
        indicator_configs: Vec<IndicatorConfig>,
        scoring_config: ScoringConfig,
        entry_actions: Vec<Box<dyn Action>>,
        monitor_actions: Vec<Box<dyn Action>>,
        exit_actions: Vec<Box<dyn Action>>,
        sizing_actions: Vec<Box<dyn Action>>,
        ticker: String,
        capital: f64,
    ) -> Self {
        Self {
            indicators,
            indicator_configs,
            scoring_config,
            entry_actions,
            monitor_actions,
            exit_actions,
            sizing_actions,
            position_manager: PositionManager::new(),
            ticker,
            capital,
            completed_trades: Vec::new(),
        }
    }

    /// process a single tick. never panics — indicator failures are caught and logged.
    /// returns a TickResult with computed scores and what event occurred.
    pub fn on_tick(&mut self, market: &MarketState) -> TickResult {
        // 1. compute all indicator scores with catch_unwind
        let mut outputs: HashMap<String, Option<f64>> = HashMap::new();
        for (id, indicator) in &self.indicators {
            let result = catch_unwind(AssertUnwindSafe(|| indicator.compute(market)));
            match result {
                Ok(Some(output)) => {
                    outputs.insert(id.clone(), Some(output.score));
                }
                Ok(None) => {
                    outputs.insert(id.clone(), None);
                }
                Err(_) => {
                    // indicator panicked — log and continue
                    warn!(indicator_id = %id, "indicator panicked, skipping");
                    outputs.insert(id.clone(), None);
                }
            }
        }

        // 2. aggregate per-timescale scores
        let mut scores = compute_timescale_scores(&outputs, &self.indicator_configs);

        // 3. compute composite score (pass indicator outputs for dynamic fusion)
        compute_composite_with_indicators(&mut scores, &self.scoring_config, Some(&outputs));

        // 4. update position if exists
        self.position_manager
            .update_on_tick(market.last_price, market.timestamp);

        // 5. evaluate actions based on position state
        let event = if self.position_manager.has_position() {
            // check monitor actions first (e.g., breakeven stop)
            for action in &self.monitor_actions {
                let signal = action.evaluate(
                    self.position_manager.current_position(),
                    market,
                    &scores,
                );
                if let ActionSignal::ModifyStop { .. } = signal {
                    // in a real system this would modify the stop
                    // for now we just note it happened
                }
            }

            // check exit actions
            let mut exit_event = TickEvent::Nothing;
            for action in &self.exit_actions {
                let signal = action.evaluate(
                    self.position_manager.current_position(),
                    market,
                    &scores,
                );
                if let ActionSignal::Exit { reason } = signal {
                    if let Some(trade) = self.position_manager.close_position(
                        market.last_price,
                        market.timestamp,
                        reason,
                    ) {
                        self.completed_trades.push(trade);
                    }
                    exit_event = TickEvent::PositionClosed;
                    break;
                }
            }
            exit_event
        } else {
            // no position — check entry actions
            let mut entry_event = TickEvent::Nothing;
            for action in &self.entry_actions {
                let signal = action.evaluate(None, market, &scores);
                if let ActionSignal::Enter {
                    direction,
                    mut size_fraction,
                    reason: _,
                } = signal
                {
                    // check sizing actions for actual size
                    for sizing in &self.sizing_actions {
                        let sizing_signal = sizing.evaluate(None, market, &scores);
                        if let ActionSignal::Enter {
                            size_fraction: sf, ..
                        } = sizing_signal
                        {
                            size_fraction = sf;
                        }
                    }

                    let _ = self.position_manager.open_position(
                        self.ticker.clone(),
                        direction,
                        market.last_price,
                        size_fraction * self.capital,
                        market.timestamp,
                    );
                    entry_event = TickEvent::PositionOpened;
                    break;
                }
            }
            entry_event
        };

        TickResult { scores, event }
    }

    pub fn completed_trades(&self) -> &[TradeRecord] {
        &self.completed_trades
    }

    pub fn has_position(&self) -> bool {
        self.position_manager.has_position()
    }

    pub fn current_position(&self) -> Option<&types::action::Position> {
        self.position_manager.current_position()
    }
}
