use engine::candle_aggregator::CandleAggregator;
use std::collections::HashMap;
use std::io::Read;

use chrono::{DateTime, Utc};
use chrono::Timelike;

use engine::TradingEngine;
use types::action::{ActionConfig, ExitReason, Position};
use types::indicator::IndicatorConfig;
use types::market::{Candle, MarketState, Timescale};
use types::scoring::{ScoringConfig, TimescaleScores};
use types::tick_result::TickEvent;

use actions::{build_actions, default_action_registry};
use indicators::{build_indicators, default_indicator_registry};

use crate::report::{compute_metrics, BacktestResult};

/// cost model for realistic backtest fills.
#[derive(Debug, Clone)]
pub struct BacktestCostConfig {
    /// slippage in basis points applied to each fill.
    pub slippage_bps: f64,
    /// half the bid-ask spread in dollars (added to buys, subtracted from sells).
    pub half_spread: f64,
    /// commission per share (0.0 for commission-free brokers like alpaca).
    pub commission_per_share: f64,
    /// SEC fee per million dollars of sell-side proceeds.
    pub sec_fee_per_million: f64,
    /// FINRA TAF per share on sell-side.
    pub finra_taf_per_share: f64,
}

impl Default for BacktestCostConfig {
    fn default() -> Self {
        Self {
            slippage_bps: 0.0,
            half_spread: 0.0,
            commission_per_share: 0.0,
            sec_fee_per_million: 0.0,
            finra_taf_per_share: 0.0,
        }
    }
}

impl BacktestCostConfig {
    /// apply cost model to an entry (buy) fill price.
    pub fn adjust_entry_price(&self, raw_price: f64) -> f64 {
        raw_price + raw_price * self.slippage_bps / 10_000.0 + self.half_spread
    }

    /// apply cost model to an exit (sell) fill price.
    pub fn adjust_exit_price(&self, raw_price: f64) -> f64 {
        raw_price - raw_price * self.slippage_bps / 10_000.0 - self.half_spread
    }

    /// compute sell-side regulatory fees for a trade.
    pub fn sell_side_fees(&self, exit_price: f64, shares: f64) -> f64 {
        let proceeds = exit_price * shares;
        let sec = proceeds / 1_000_000.0 * self.sec_fee_per_million;
        let taf = shares * self.finra_taf_per_share;
        let commission = shares * self.commission_per_share;
        sec + taf + commission
    }
}

/// configuration for a backtest run.
#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub ticker: String,
    pub initial_capital: f64,
    pub indicator_configs: Vec<IndicatorConfig>,
    pub action_configs: Vec<ActionConfig>,
    pub scoring_config: ScoringConfig,
    /// optional cost model. defaults to zero costs if None.
    pub cost_config: Option<BacktestCostConfig>,
    /// optional session config for enforcing time/position constraints.
    pub session_config: Option<types::config::SessionConfig>,
    /// per-window exit overrides (window name → exit params).
    pub window_exit_overrides: HashMap<String, engine::WindowExitOverrides>,
}

/// historical candle data for replay, keyed by timescale.
#[derive(Debug, Clone)]
pub struct BacktestData {
    pub candles: HashMap<Timescale, Vec<Candle>>,
    /// which timescale drives the tick loop. each candle in this
    /// timescale produces one tick.
    pub primary_timescale: Timescale,
}

/// rolling candle window per timescale, matching the live `MarketStateBuilder::new(200)`.
pub const LIVE_CANDLE_WINDOW: usize = 200;

/// parse "HH:MM" to minutes since midnight.
fn parse_hm_to_minutes(s: &str) -> Option<u32> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 2 {
        let h: u32 = parts[0].parse().ok()?;
        let m: u32 = parts[1].parse().ok()?;
        Some(h * 60 + m)
    } else {
        None
    }
}

/// check if a timestamp (in UTC) is within the session constraint window.
/// session config times are in US Eastern; convert UTC timestamp before comparing.
/// returns true if entry is allowed at this time.
fn entry_allowed(
    timestamp: &DateTime<chrono::Utc>,
    session: &types::config::SessionConfig,
    first_candle_ts: &DateTime<chrono::Utc>,
) -> bool {
    // avoid_first_minutes: skip entries in the first N minutes of the session
    let elapsed_minutes = (*timestamp - *first_candle_ts).num_minutes();
    if elapsed_minutes < session.avoid_first_minutes as i64 {
        return false;
    }

    // no_new_entries_after: convert UTC to Eastern, then compare
    if let Some(cutoff_minutes) = parse_hm_to_minutes(&session.no_new_entries_after) {
        let eastern = chrono_tz::US::Eastern;
        let local = timestamp.with_timezone(&eastern);
        let current_minutes = local.hour() * 60 + local.minute();
        if current_minutes >= cutoff_minutes {
            return false;
        }
    }

    true
}

/// a point on the tick-level equity curve, used for intra-trade drawdown tracking.
#[derive(Debug, Clone)]
pub struct TickEquityPoint {
    pub timestamp: DateTime<chrono::Utc>,
    pub equity: f64,
}

/// run a backtest: replay historical data through a trading engine and collect results.
pub fn run_backtest(config: &BacktestConfig, data: &BacktestData) -> Result<BacktestResult, String> {
    let ind_reg = default_indicator_registry();
    let indicators = build_indicators(&config.indicator_configs, &ind_reg)?;

    let act_reg = default_action_registry();
    let actions = build_actions(&config.action_configs, &act_reg)?;

    let mut engine = TradingEngine::new(
        indicators,
        config.indicator_configs.clone(),
        config.scoring_config.clone(),
        actions.entry,
        actions.monitor,
        actions.exit,
        actions.sizing,
        config.ticker.clone(),
        config.initial_capital,
        config.session_config.clone(),
    );

    // extract max_hold_ms from action configs for position context meta-indicators
    let max_hold_ms = config
        .action_configs
        .iter()
        .find(|a| a.action_type == "max_hold_timeout")
        .and_then(|a| a.params.get("max_hold_ms"))
        .and_then(|v| v.as_i64())
        .unwrap_or(2_700_000);
    engine.set_max_hold_ms(max_hold_ms);

    // apply per-window exit overrides
    if !config.window_exit_overrides.is_empty() {
        engine.set_window_exit_overrides(config.window_exit_overrides.clone());
    }

    let cost = config.cost_config.clone().unwrap_or_default();

    let primary_candles = data
        .candles
        .get(&data.primary_timescale)
        .ok_or_else(|| format!("no candles for primary timescale {:?}", data.primary_timescale))?;

    if primary_candles.is_empty() {
        return Err("no candle data provided".to_string());
    }

    let start_time = primary_candles.first().unwrap().timestamp;
    let end_time = primary_candles.last().unwrap().timestamp;

    // same rolling-window size as MarketStateBuilder::new(200) in the live trader
    let mut aggregator = if data.primary_timescale == Timescale::OneMinute {
        Some(CandleAggregator::new(LIVE_CANDLE_WINDOW))
    } else {
        None
    };

    // track entry scores per position so we can pair them with exit scores
    let mut pending_entry_scores: Option<TimescaleScores> = None;
    let mut trade_scores: Vec<(TimescaleScores, TimescaleScores)> = Vec::new();
    // track entry reasons per trade (e.g. "window:candle_reversal")
    let mut pending_entry_reason: Option<String> = None;
    let mut trade_entry_reasons: Vec<String> = Vec::new();

    // deferred fill state for next-bar execution.
    // when the engine signals an entry/exit, we undo it and re-execute on the next bar.
    struct DeferredEntry {
        position: Position,
        scores: TimescaleScores,
        entry_reason: String,
    }
    struct DeferredExit {
        exit_reason: ExitReason,
        scores: TimescaleScores,
    }
    let mut deferred_entry: Option<DeferredEntry> = None;
    let mut deferred_exit: Option<DeferredExit> = None;

    // cumulative VWAP tracking
    let mut cum_tp_vol = 0.0_f64; // sum(typical_price * volume)
    let mut cum_vol = 0.0_f64;    // sum(volume)

    // tick-level equity tracking for drawdown
    let mut tick_equity_curve: Vec<TickEquityPoint> = Vec::new();
    let mut realized_pnl = 0.0_f64;

    // score diagnostics for no-trade day analysis
    let mut max_composite = f64::NEG_INFINITY;
    let mut max_composite_time: Option<DateTime<Utc>> = None;
    let mut positive_score_ticks: usize = 0;
    let mut total_ticks: usize = 0;

    // replay tick by tick: at each tick, provide candles up to that point
    for i in 1..=primary_candles.len() {
        let last_candle = &primary_candles[i - 1];

        // update cumulative VWAP
        let typical_price = (last_candle.high + last_candle.low + last_candle.close) / 3.0;
        cum_tp_vol += typical_price * last_candle.volume;
        cum_vol += last_candle.volume;
        let session_vwap = if cum_vol > 0.0 { cum_tp_vol / cum_vol } else { last_candle.close };

        // execute deferred fills from previous bar's signals at this bar's open
        if let Some(entry) = deferred_entry.take() {
            let adjusted_fill = cost.adjust_entry_price(last_candle.open);
            let _ = engine.force_open_position(
                entry.position.ticker,
                entry.position.direction,
                adjusted_fill,
                entry.position.size,
                last_candle.timestamp,
            );
            pending_entry_scores = Some(entry.scores);
            pending_entry_reason = Some(entry.entry_reason);
        }

        if let Some(exit) = deferred_exit.take() {
            let adjusted_fill = cost.adjust_exit_price(last_candle.open);
            if let Some(trade) = engine.force_close_position(
                adjusted_fill,
                last_candle.timestamp,
                exit.exit_reason,
            ) {
                realized_pnl += trade.pnl;
                let entry_scores = pending_entry_scores.take().unwrap_or_default();
                trade_scores.push((entry_scores, exit.scores));
                trade_entry_reasons.push(pending_entry_reason.take().unwrap_or_default());
            }
        }

        // build the per-timescale windows exactly as the live engine does: feed the
        // 1-minute bar through the same CandleAggregator, which yields completed
        // 5m/1h candles plus the IN-PROGRESS candle built only from bars seen so far.
        //
        // (the previous implementation pre-aggregated the whole series and included
        // any candle whose bucket had *started* — so at 09:31 the engine saw the
        // completed 09:30–09:34 five-minute candle and the 09:30–10:29 hourly
        // candle, closes included. every backtest result before 2026-09-11 has
        // that look-ahead in it.)
        let candle_map: HashMap<Timescale, Vec<Candle>> = if let Some(agg) = aggregator.as_mut() {
            agg.on_candle(last_candle.clone());
            agg.candle_windows()
        } else {
            // non-1-minute primary (legacy csv mode): only the primary series is
            // available, growing window, no higher timescales.
            let mut m = HashMap::new();
            m.insert(data.primary_timescale, primary_candles[..i].to_vec());
            m
        };

        let mut market = MarketState {
            last_price: last_candle.close,
            bid: last_candle.close - 0.01,
            ask: last_candle.close + 0.01,
            timestamp: last_candle.timestamp,
            candles: candle_map,
            spread: 0.02,
            session_vwap,
            session_volume: cum_vol,
            position_context: None,
            session_progress: None,
            entries_blocked: false,
            total_deployed_capital: None,
            total_initial_capital: None,
            index_return: None,
            cross_ticker_correlation: None,
        };

        let result = engine.on_tick(&mut market);

        // track score diagnostics
        total_ticks += 1;
        if result.scores.composite > max_composite {
            max_composite = result.scores.composite;
            max_composite_time = Some(last_candle.timestamp);
        }
        if result.scores.composite > 0.0 {
            positive_score_ticks += 1;
        }

        match &result.event {
            TickEvent::PositionOpened => {
                if i < primary_candles.len() {
                    // undo the position and defer to next bar
                    let entry_blocked = config.session_config.as_ref().is_some_and(|sc| {
                        !entry_allowed(&last_candle.timestamp, sc, &start_time)
                    });

                    if let Some(pos) = engine.undo_last_open() {
                        if !entry_blocked {
                            deferred_entry = Some(DeferredEntry {
                                position: pos,
                                scores: result.scores.clone(),
                                entry_reason: result.entry_reason.clone(),
                            });
                        }
                        // if blocked, the undo already removed it — signal dropped
                    }
                }
                // if last bar, signal is dropped (no next bar to fill).
                // undo the position since we can't defer.
                else {
                    engine.undo_last_open();
                }
            }
            TickEvent::PositionClosed => {
                if i < primary_candles.len() {
                    // undo the close and defer to next bar
                    if let Some(trade) = engine.undo_last_close() {
                        deferred_exit = Some(DeferredExit {
                            exit_reason: trade.exit_reason.clone(),
                            scores: result.scores.clone(),
                        });
                    }
                } else {
                    // last bar — exit stays at close (can't defer)
                    let entry_scores = pending_entry_scores.take().unwrap_or_default();
                    trade_scores.push((entry_scores, result.scores.clone()));
                    trade_entry_reasons.push(pending_entry_reason.take().unwrap_or_default());
                    if let Some(trade) = engine.completed_trades().last() {
                        realized_pnl += trade.pnl;
                    }
                }
            }
            TickEvent::Nothing => {}
        }

        // track tick-level equity (realized + unrealized)
        let unrealized = engine
            .current_position()
            .map(|p| p.unrealized_pnl)
            .unwrap_or(0.0);
        tick_equity_curve.push(TickEquityPoint {
            timestamp: last_candle.timestamp,
            equity: config.initial_capital + realized_pnl + unrealized,
        });
    }

    // drop any deferred signals that never got filled (data ended)

    let trades = engine.completed_trades().to_vec();
    let (metrics, equity_curve) =
        compute_metrics(&trades, config.initial_capital, start_time, end_time, &tick_equity_curve);

    // normalize max_composite if no ticks processed
    if max_composite == f64::NEG_INFINITY {
        max_composite = 0.0;
    }

    Ok(BacktestResult {
        config_id: "backtest".to_string(),
        ticker: config.ticker.clone(),
        start_time,
        end_time,
        initial_capital: config.initial_capital,
        trades,
        trade_scores,
        trade_entry_reasons,
        equity_curve,
        metrics,
        max_composite,
        max_composite_time,
        positive_score_ticks,
        total_ticks,
    })
}

/// parse candle data from CSV format: timestamp_epoch,open,high,low,close,volume
pub fn load_candles_from_csv<R: Read>(reader: R) -> Result<Vec<Candle>, String> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(reader);

    let mut candles = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| format!("csv parse error: {}", e))?;
        if record.len() < 6 {
            return Err(format!("expected 6 fields, got {}", record.len()));
        }

        let ts_epoch: i64 = record[0]
            .parse()
            .map_err(|e| format!("bad timestamp: {}", e))?;
        let timestamp = DateTime::from_timestamp(ts_epoch, 0)
            .ok_or_else(|| format!("invalid timestamp: {}", ts_epoch))?;

        let open: f64 = record[1].parse().map_err(|e| format!("bad open: {}", e))?;
        let high: f64 = record[2].parse().map_err(|e| format!("bad high: {}", e))?;
        let low: f64 = record[3].parse().map_err(|e| format!("bad low: {}", e))?;
        let close: f64 = record[4].parse().map_err(|e| format!("bad close: {}", e))?;
        let volume: f64 = record[5]
            .parse()
            .map_err(|e| format!("bad volume: {}", e))?;

        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        });
    }

    Ok(candles)
}
