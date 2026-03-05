use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use chrono::{DateTime, Timelike, Utc};
use tracing::warn;
use types::action::{Action, ActionSignal, ExitReason};
use types::config::SessionConfig;
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
    available_capital: f64,
    completed_trades: Vec<TradeRecord>,
    /// when set, fills use this price instead of market.last_price.
    /// used by backtest for next-bar execution (fill at next bar's open).
    fill_price_override: Option<f64>,
    /// session constraints (avoid_first_minutes, no_new_entries_after, cooldown, circuit breaker).
    session_config: Option<SessionConfig>,
    /// timestamp of the first tick seen (for avoid_first_minutes).
    session_start_time: Option<DateTime<Utc>>,
    /// timestamp when the last position was closed (for re-entry cooldown).
    last_exit_time: Option<DateTime<Utc>>,
    /// cumulative realized P&L for daily loss circuit breaker.
    cumulative_realized_pnl: f64,
    /// true when daily loss breaker has been triggered.
    daily_loss_breaker_active: bool,
    /// snapshot of initial capital for circuit breaker percentage calculation.
    initial_capital_snapshot: f64,
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
        session_config: Option<SessionConfig>,
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
            available_capital: capital,
            completed_trades: Vec::new(),
            fill_price_override: None,
            session_config,
            session_start_time: None,
            last_exit_time: None,
            cumulative_realized_pnl: 0.0,
            daily_loss_breaker_active: false,
            initial_capital_snapshot: capital,
        }
    }

    /// process a single tick. never panics — indicator failures are caught and logged.
    /// returns a TickResult with computed scores and what event occurred.
    pub fn on_tick(&mut self, market: &MarketState) -> TickResult {
        // track session start time
        if self.session_start_time.is_none() {
            self.session_start_time = Some(market.timestamp);
        }

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
        let fill_price = self.fill_price_override.unwrap_or(market.last_price);

        let event = if self.position_manager.has_position() {
            // check score-based exit first (before action-based exits)
            if scores.composite <= self.scoring_config.exit_threshold {
                if let Some(trade) = self.position_manager.close_position(
                    fill_price,
                    market.timestamp,
                    ExitReason::ScoreExit,
                ) {
                    self.available_capital += trade.size + trade.pnl;
                    self.record_exit(&trade, market.timestamp);
                    self.completed_trades.push(trade);
                    return TickResult { scores, event: TickEvent::PositionClosed };
                }
            }

            // check monitor actions (e.g., breakeven stop)
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
                        fill_price,
                        market.timestamp,
                        reason,
                    ) {
                        self.available_capital += trade.size + trade.pnl;
                        self.record_exit(&trade, market.timestamp);
                        self.completed_trades.push(trade);
                    }
                    exit_event = TickEvent::PositionClosed;
                    break;
                }
            }
            exit_event
        } else {
            // no position — check if entries are allowed
            if self.is_entry_blocked(market) {
                return TickResult { scores, event: TickEvent::Nothing };
            }

            // check entry actions
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

                    // correlation-aware sizing: cap total exposure
                    if let (Some(deployed), Some(total_cap)) =
                        (market.total_deployed_capital, market.total_initial_capital)
                    {
                        let max_exposure = self
                            .session_config
                            .as_ref()
                            .map(|sc| sc.max_capital_deployed_pct)
                            .unwrap_or(1.0);
                        let proposed = size_fraction * self.available_capital;
                        if (deployed + proposed) / total_cap > max_exposure {
                            let remaining = max_exposure * total_cap - deployed;
                            if remaining <= 0.0 {
                                break; // skip entry
                            }
                            size_fraction = remaining / self.available_capital;
                        }
                    }

                    let position_size = size_fraction * self.available_capital;
                    let _ = self.position_manager.open_position(
                        self.ticker.clone(),
                        direction,
                        fill_price,
                        position_size,
                        market.timestamp,
                    );
                    self.available_capital -= position_size;
                    entry_event = TickEvent::PositionOpened;
                    break;
                }
            }
            entry_event
        };

        TickResult { scores, event }
    }

    /// check all entry-blocking conditions.
    fn is_entry_blocked(&self, market: &MarketState) -> bool {
        // entries_blocked flag from cross-ticker position limit
        if market.entries_blocked {
            return true;
        }

        // daily loss circuit breaker
        if self.daily_loss_breaker_active {
            return true;
        }

        if let Some(ref sc) = self.session_config {
            // avoid_first_minutes
            if let Some(start) = self.session_start_time {
                let elapsed = (market.timestamp - start).num_minutes();
                if elapsed < sc.avoid_first_minutes as i64 {
                    return true;
                }
            }

            // no_new_entries_after
            if let Some(cutoff) = sc.no_new_entries_after_minutes() {
                let eastern = chrono_tz::US::Eastern;
                let local = market.timestamp.with_timezone(&eastern);
                let current_minutes = local.hour() * 60 + local.minute();
                if current_minutes >= cutoff {
                    return true;
                }
            }

            // re-entry cooldown
            if sc.entry_cooldown_ms > 0 {
                if let Some(last_exit) = self.last_exit_time {
                    let elapsed = (market.timestamp - last_exit).num_milliseconds();
                    if elapsed < sc.entry_cooldown_ms {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// record state changes after a position exits.
    fn record_exit(&mut self, trade: &TradeRecord, timestamp: DateTime<Utc>) {
        self.last_exit_time = Some(timestamp);
        self.cumulative_realized_pnl += trade.pnl;

        // check if daily loss breaker should activate
        if let Some(ref sc) = self.session_config {
            if let Some(max_loss) = sc.max_daily_loss_pct {
                if self.initial_capital_snapshot > 0.0
                    && -self.cumulative_realized_pnl / self.initial_capital_snapshot >= max_loss
                {
                    self.daily_loss_breaker_active = true;
                }
            }
        }
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

    /// set a fill price override for the next on_tick call.
    /// used by backtest for next-bar execution.
    pub fn set_fill_price_override(&mut self, price: Option<f64>) {
        self.fill_price_override = price;
    }

    /// current available capital (initial minus deployed in open positions).
    pub fn available_capital(&self) -> f64 {
        self.available_capital
    }

    /// undo the last position open (for next-bar execution in backtest).
    /// returns the cancelled position details if one was open.
    pub fn undo_last_open(&mut self) -> Option<types::action::Position> {
        let pos = self.position_manager.cancel_position()?;
        self.available_capital += pos.size;
        Some(pos)
    }

    /// undo the last trade close (for next-bar execution in backtest).
    /// re-opens the position from the removed trade record.
    pub fn undo_last_close(&mut self) -> Option<TradeRecord> {
        let trade = self.completed_trades.pop()?;
        self.available_capital -= trade.size;
        let _ = self.position_manager.open_position(
            trade.ticker.clone(),
            trade.direction,
            trade.entry_price,
            trade.size,
            trade.entry_time,
        );
        Some(trade)
    }

    /// force open a position at a specific price (for deferred backtest fills).
    pub fn force_open_position(
        &mut self,
        ticker: String,
        direction: types::action::TradeDirection,
        price: f64,
        size: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), String> {
        self.position_manager.open_position(ticker, direction, price, size, timestamp)?;
        self.available_capital -= size;
        Ok(())
    }

    /// force close the current position at a specific price (for deferred backtest fills).
    pub fn force_close_position(
        &mut self,
        price: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
        reason: types::action::ExitReason,
    ) -> Option<TradeRecord> {
        let trade = self.position_manager.close_position(price, timestamp, reason)?;
        self.available_capital += trade.size + trade.pnl;
        self.record_exit(&trade, timestamp);
        self.completed_trades.push(trade.clone());
        Some(trade)
    }

    /// cumulative realized P&L this session.
    pub fn cumulative_realized_pnl(&self) -> f64 {
        self.cumulative_realized_pnl
    }

    /// whether the daily loss circuit breaker has been triggered.
    pub fn is_daily_loss_breaker_active(&self) -> bool {
        self.daily_loss_breaker_active
    }
}
