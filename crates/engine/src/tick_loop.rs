use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

use actions::exit::session_close::eastern_minutes;
use chrono::{DateTime, NaiveDate, TimeZone, Timelike, Utc};
use tracing::warn;
use types::action::{Action, ActionSignal, ExitReason, TradeDirection};
use types::config::SessionConfig;
use types::indicator::{Indicator, IndicatorConfig};
use types::market::{MarketState, PositionContext};
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
    /// exchange-local (US/Eastern) calendar date of the current trading session.
    /// when a tick arrives with a different date, per-day state is reset.
    current_session_date: Option<NaiveDate>,
    /// 09:30 US/Eastern of the current session date, in UTC (for avoid_first_minutes).
    session_start_time: Option<DateTime<Utc>>,
    /// timestamp when the last position was closed (for re-entry cooldown).
    last_exit_time: Option<DateTime<Utc>>,
    /// cumulative realized P&L for daily loss circuit breaker.
    cumulative_realized_pnl: f64,
    /// true when daily loss breaker has been triggered.
    daily_loss_breaker_active: bool,
    /// snapshot of initial capital for circuit breaker percentage calculation.
    initial_capital_snapshot: f64,
    /// max hold time in ms, used to populate PositionContext for meta-indicators.
    max_hold_ms: i64,
    /// default max hold (stored so we can restore after per-window overrides).
    default_max_hold_ms: i64,
    /// per-window exit overrides: maps window name → (score_exit_threshold, max_hold_ms).
    /// score_exit_threshold of f64::MAX means "disabled".
    window_exit_overrides: HashMap<String, WindowExitOverrides>,
    /// active score exit threshold (may differ from config when a window override is active).
    active_score_exit_threshold: f64,
    /// stop price installed by a monitor action (`ModifyStop`) for the open position.
    /// long: exit when price <= stop; short: exit when price >= stop. cleared when flat.
    /// before 2026-09-12 `ModifyStop` was received and ignored, so `breakeven_stop`
    /// never did anything in five years of backtests.
    monitor_stop_price: Option<f64>,
}

/// per-window exit parameter overrides.
#[derive(Debug, Clone)]
pub struct WindowExitOverrides {
    /// override for score exit threshold. f64::MAX = disabled.
    pub score_exit_threshold: f64,
    /// override for max hold time in ms.
    pub max_hold_ms: i64,
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
        let exit_threshold = scoring_config.exit_threshold;
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
            current_session_date: None,
            session_start_time: None,
            last_exit_time: None,
            cumulative_realized_pnl: 0.0,
            daily_loss_breaker_active: false,
            initial_capital_snapshot: capital,
            max_hold_ms: 2_700_000, // default 45 min, overridable via set_max_hold_ms
            default_max_hold_ms: 2_700_000,
            window_exit_overrides: HashMap::new(),
            active_score_exit_threshold: exit_threshold,
            monitor_stop_price: None,
        }
    }

    /// register per-window exit overrides.
    pub fn set_window_exit_overrides(&mut self, overrides: HashMap<String, WindowExitOverrides>) {
        self.window_exit_overrides = overrides;
    }

    /// process a single tick. never panics — indicator failures are caught and logged.
    /// returns a TickResult with computed scores and what event occurred.
    pub fn on_tick(&mut self, market: &mut MarketState) -> TickResult {
        if self.position_manager.current_position().is_none() {
            self.monitor_stop_price = None;
        }
        // 0. detect a new trading day (US/Eastern) and reset per-day state.
        //    this makes avoid_first_minutes count from the 09:30 open rather than
        //    from process start, and makes the daily loss breaker actually daily.
        let et_date = market
            .timestamp
            .with_timezone(&chrono_tz::US::Eastern)
            .date_naive();
        if self.current_session_date != Some(et_date) {
            self.start_new_session(et_date, market.timestamp);
        }

        // 1. inject position context from current (stale) position state
        //    before indicators so meta-indicators can use it.
        //    uses previous tick's position data — one tick of lag is acceptable.
        market.position_context = self.position_manager.current_position().map(|pos| {
            PositionContext {
                direction: match pos.direction {
                    TradeDirection::Long => 1.0,
                    TradeDirection::Short => -1.0,
                },
                unrealized_pnl_pct: pos.unrealized_pnl_pct,
                hold_duration_ms: pos.hold_duration_ms,
                max_hold_ms: self.max_hold_ms,
            }
        });

        // 2. compute all indicator scores with catch_unwind
        let mut outputs: HashMap<String, Option<f64>> = HashMap::new();
        for (id, indicator) in &self.indicators {
            let result = catch_unwind(AssertUnwindSafe(|| indicator.compute(market)));
            match result {
                Ok(Some(output)) => {
                    outputs.insert(id.clone(), Some(output.score));
                    // propagate indicator metadata as {id}.{key} entries
                    // (enables downstream consumers to read e.g. candle_5min.pattern_name)
                    for (key, value) in &output.metadata {
                        outputs.insert(format!("{}.{}", id, key), Some(*value));
                    }
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

        // 3. aggregate per-timescale scores
        let mut scores = compute_timescale_scores(&outputs, &self.indicator_configs);

        // 3b. attach per-indicator scores for entry window conditions
        scores.indicator_scores = Some(outputs.clone());

        // 4. compute composite score (pass indicator outputs for dynamic fusion)
        compute_composite_with_indicators(&mut scores, &self.scoring_config, Some(&outputs));

        // 5. update position state (preserves original timing for exits)
        self.position_manager
            .update_on_tick(market.last_price, market.timestamp);

        // 6. evaluate actions based on position state
        let fill_price = self.fill_price_override.unwrap_or(market.last_price);
        let mut entry_reason_out = String::new();
        let mut blocked_by_out: Option<String> = None;
        let mut near_miss_out: Option<String> = None;

        let event = if self.position_manager.has_position() {
            // session force-exit safety net: independent of whether a
            // session_close action is configured, never hold past force_exit_by (ET).
            if let Some(cutoff) = self
                .session_config
                .as_ref()
                .and_then(|sc| sc.force_exit_by_minutes())
            {
                if eastern_minutes(market.timestamp) >= cutoff {
                    if let Some(trade) = self.position_manager.close_position(
                        fill_price,
                        market.timestamp,
                        ExitReason::SessionClose,
                    ) {
                        self.available_capital += trade.size * trade.entry_price + trade.pnl;
                        self.record_exit(&trade, market.timestamp);
                        self.completed_trades.push(trade);
                        return TickResult {
                            scores,
                            event: TickEvent::PositionClosed,
                            entry_reason: String::new(),
                            entry_blocked_by: None,
                            near_miss: None,
                        };
                    }
                }
            }

            // check score-based exit first (before action-based exits)
            // uses per-window override if active, otherwise config default.
            // direction-aware: a long exits when the composite falls to the threshold,
            // a short exits when it rises to the mirrored threshold.
            let direction = self
                .position_manager
                .current_position()
                .map(|p| p.direction)
                .unwrap_or(TradeDirection::Long);
            let score_exit_hit = match direction {
                TradeDirection::Long => scores.composite <= self.active_score_exit_threshold,
                TradeDirection::Short => scores.composite >= -self.active_score_exit_threshold,
            };
            if score_exit_hit {
                // hourly exit override: suppress ScoreExit when hourly trend is
                // strong (in the position's direction) and position is profitable.
                let hourly_override = self.scoring_config.hourly_exit_override.is_some_and(|thresh| {
                    let h = scores.one_hour.unwrap_or(0.0);
                    let trend_ok = match direction {
                        TradeDirection::Long => h >= thresh,
                        TradeDirection::Short => h <= -thresh,
                    };
                    trend_ok
                        && self.position_manager.current_position()
                            .is_some_and(|pos| pos.unrealized_pnl > 0.0)
                });

                if !hourly_override {
                    if let Some(trade) = self.position_manager.close_position(
                        fill_price,
                        market.timestamp,
                        ExitReason::ScoreExit,
                    ) {
                        self.available_capital += trade.size * trade.entry_price + trade.pnl;
                        self.record_exit(&trade, market.timestamp);
                        self.completed_trades.push(trade);
                        return TickResult {
                            scores,
                            event: TickEvent::PositionClosed,
                            entry_reason: String::new(),
                            entry_blocked_by: None,
                            near_miss: None,
                        };
                    }
                }
            }

            // check monitor actions (e.g., breakeven stop). a ModifyStop installs a
            // stop that only ever tightens (long: max, short: min).
            for action in &self.monitor_actions {
                let signal = action.evaluate(
                    self.position_manager.current_position(),
                    market,
                    &scores,
                );
                if let ActionSignal::ModifyStop { new_stop_price } = signal {
                    let tightened = match (self.monitor_stop_price, direction) {
                        (None, _) => new_stop_price,
                        (Some(cur), TradeDirection::Long) => cur.max(new_stop_price),
                        (Some(cur), TradeDirection::Short) => cur.min(new_stop_price),
                    };
                    self.monitor_stop_price = Some(tightened);
                }
            }

            // monitor-installed stop hit?
            if let Some(stop) = self.monitor_stop_price {
                let hit = match direction {
                    TradeDirection::Long => market.last_price <= stop,
                    TradeDirection::Short => market.last_price >= stop,
                };
                if hit {
                    if let Some(trade) = self.position_manager.close_position(
                        fill_price,
                        market.timestamp,
                        ExitReason::BreakevenStop,
                    ) {
                        self.available_capital += trade.size * trade.entry_price + trade.pnl;
                        self.record_exit(&trade, market.timestamp);
                        self.completed_trades.push(trade);
                        return TickResult {
                            scores,
                            event: TickEvent::PositionClosed,
                            entry_reason: String::new(),
                            entry_blocked_by: None,
                            near_miss: None,
                        };
                    }
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
                        self.available_capital += trade.size * trade.entry_price + trade.pnl;
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
            if let Some(reason) = self.entry_block_reason(market) {
                return TickResult {
                    scores,
                    event: TickEvent::Nothing,
                    entry_reason: String::new(),
                    entry_blocked_by: Some(reason.to_string()),
                    near_miss: None,
                };
            }

            // check entry actions (sorted by priority — reject gates first, then windows)
            let mut entry_event = TickEvent::Nothing;
            for action in &self.entry_actions {
                let signal = action.evaluate(None, market, &scores);
                if matches!(signal, ActionSignal::RejectEntry) {
                    // reject gate fired — no entry this tick
                    blocked_by_out = Some(format!("reject_gate:{}", action.name()));
                    break;
                }
                if let ActionSignal::Enter {
                    direction,
                    mut size_fraction,
                    reason,
                } = signal
                {
                    // apply per-window exit overrides if this is a window entry
                    if let Some(window_name) = reason.strip_prefix("window:") {
                        if let Some(ovr) = self.window_exit_overrides.get(window_name) {
                            self.active_score_exit_threshold = ovr.score_exit_threshold;
                            self.max_hold_ms = ovr.max_hold_ms;
                        }
                    }
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

                    // hard safety clamp on position size, independent of sizing actions.
                    // a mis-set fraction must never drive available_capital negative.
                    let max_pos = self
                        .session_config
                        .as_ref()
                        .and_then(|sc| sc.max_position_pct)
                        .unwrap_or(1.0)
                        .clamp(0.0, 1.0);
                    if !size_fraction.is_finite() || size_fraction <= 0.0 {
                        warn!(
                            ticker = %self.ticker,
                            size_fraction,
                            "sizing produced a non-positive fraction, skipping entry"
                        );
                        break;
                    }
                    if size_fraction > max_pos {
                        warn!(
                            ticker = %self.ticker,
                            requested = size_fraction,
                            clamped_to = max_pos,
                            "size_fraction exceeds max_position_pct, clamping"
                        );
                        size_fraction = max_pos;
                    }
                    if fill_price <= 0.0 || !fill_price.is_finite() || self.available_capital <= 0.0 {
                        warn!(
                            ticker = %self.ticker,
                            fill_price,
                            available_capital = self.available_capital,
                            "cannot size position, skipping entry"
                        );
                        break;
                    }

                    // whole shares only — this is what the broker will actually fill,
                    // so the engine's P&L must be computed on the same quantity.
                    let requested_dollars = size_fraction * self.available_capital;
                    let num_shares = (requested_dollars / fill_price).floor();
                    if num_shares < 1.0 {
                        warn!(
                            ticker = %self.ticker,
                            requested_dollars,
                            fill_price,
                            "position would be less than one share, skipping entry"
                        );
                        break;
                    }
                    let position_dollars = num_shares * fill_price;
                    if self
                        .position_manager
                        .open_position(
                            self.ticker.clone(),
                            direction,
                            fill_price,
                            num_shares,
                            market.timestamp,
                        )
                        .is_ok()
                    {
                        self.available_capital -= position_dollars;
                        entry_event = TickEvent::PositionOpened;
                        entry_reason_out = reason;
                    }
                    break;
                }
            }

            // no entry and no gate: ask entry actions why (near-miss diagnostics)
            if matches!(entry_event, TickEvent::Nothing) && blocked_by_out.is_none() {
                let parts: Vec<String> = self
                    .entry_actions
                    .iter()
                    .filter_map(|a| a.diagnose(market, &scores))
                    .collect();
                if !parts.is_empty() {
                    near_miss_out = Some(parts.join(" | "));
                }
            }
            entry_event
        };

        TickResult {
            scores,
            event,
            entry_reason: entry_reason_out,
            entry_blocked_by: blocked_by_out,
            near_miss: near_miss_out,
        }
    }

    /// reset per-day state at the first tick of a new exchange-local trading day.
    fn start_new_session(&mut self, et_date: NaiveDate, tick_ts: DateTime<Utc>) {
        let open = et_date
            .and_hms_opt(9, 30, 0)
            .and_then(|naive| chrono_tz::US::Eastern.from_local_datetime(&naive).single())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(tick_ts);
        if self.current_session_date.is_some() {
            tracing::info!(
                ticker = %self.ticker,
                date = %et_date,
                realized_pnl_prev_day = self.cumulative_realized_pnl,
                breaker_was_active = self.daily_loss_breaker_active,
                "new trading session, resetting daily state"
            );
        }
        self.current_session_date = Some(et_date);
        self.session_start_time = Some(open);
        self.cumulative_realized_pnl = 0.0;
        self.daily_loss_breaker_active = false;
        self.last_exit_time = None;
    }

    /// check all entry-blocking conditions. returns the first gate that blocks,
    /// or `None` when entries may be evaluated.
    pub fn entry_block_reason(&self, market: &MarketState) -> Option<&'static str> {
        // entries_blocked flag from cross-ticker position limit
        if market.entries_blocked {
            return Some("entries_blocked");
        }

        // daily loss circuit breaker
        if self.daily_loss_breaker_active {
            return Some("daily_loss_breaker");
        }

        if let Some(ref sc) = self.session_config {
            // avoid_first_minutes — measured from the 09:30 ET open of the
            // current session. pre-open ticks have negative elapsed and are blocked.
            if let Some(start) = self.session_start_time {
                let elapsed = (market.timestamp - start).num_minutes();
                if elapsed < sc.avoid_first_minutes as i64 {
                    return Some("avoid_first_minutes");
                }
            }

            // no_new_entries_after
            if let Some(cutoff) = sc.no_new_entries_after_minutes() {
                let eastern = chrono_tz::US::Eastern;
                let local = market.timestamp.with_timezone(&eastern);
                let current_minutes = local.hour() * 60 + local.minute();
                if current_minutes >= cutoff {
                    return Some("no_new_entries_after");
                }
            }

            // re-entry cooldown
            if sc.entry_cooldown_ms > 0 {
                if let Some(last_exit) = self.last_exit_time {
                    let elapsed = (market.timestamp - last_exit).num_milliseconds();
                    if elapsed < sc.entry_cooldown_ms {
                        return Some("entry_cooldown");
                    }
                }
            }
        }

        None
    }

    /// record state changes after a position exits.
    fn record_exit(&mut self, trade: &TradeRecord, timestamp: DateTime<Utc>) {
        self.last_exit_time = Some(timestamp);
        self.cumulative_realized_pnl += trade.pnl;

        // reset per-window exit overrides to defaults
        self.active_score_exit_threshold = self.scoring_config.exit_threshold;
        self.max_hold_ms = self.default_max_hold_ms;

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

    /// set the max hold time for position context meta-indicators.
    pub fn set_max_hold_ms(&mut self, ms: i64) {
        self.max_hold_ms = ms;
        self.default_max_hold_ms = ms;
    }

    /// current available capital (initial minus deployed in open positions).
    pub fn available_capital(&self) -> f64 {
        self.available_capital
    }

    /// undo the last position open (for next-bar execution in backtest).
    /// returns the cancelled position details if one was open.
    pub fn undo_last_open(&mut self) -> Option<types::action::Position> {
        let pos = self.position_manager.cancel_position()?;
        self.available_capital += pos.size * pos.entry_price;
        Some(pos)
    }

    /// undo the last trade close (for next-bar execution in backtest).
    /// re-opens the position from the removed trade record.
    pub fn undo_last_close(&mut self) -> Option<TradeRecord> {
        let trade = self.completed_trades.pop()?;
        self.available_capital -= trade.size * trade.entry_price;
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
        self.available_capital -= size * price;
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
        self.available_capital += trade.size * trade.entry_price + trade.pnl;
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

    /// exchange-local date of the session the engine is currently in, if any tick has been seen.
    pub fn current_session_date(&self) -> Option<NaiveDate> {
        self.current_session_date
    }

    /// the engine's ticker symbol.
    pub fn ticker(&self) -> &str {
        &self.ticker
    }
}
