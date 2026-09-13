//! paper_trader — live market data → trading engine → (simulated | alpaca paper) broker → postgres.
//!
//! operational guarantees this binary is responsible for (see docs/paper_trading_plan_2026-09.md):
//! - never trades on a synthetic feed unless `--demo` is passed explicitly
//! - only regular-trading-hours bars reach the engine
//! - every open position is closed at the broker (and recorded) on shutdown,
//!   on config swap, and if the session close is missed for lack of bars
//! - config hot-reload is deferred per ticker until that ticker is flat
//! - live state (scores, positions, gates, heartbeat) is persisted to
//!   `engine_state` / `entry_block_events` so an outside observer can see it

use std::collections::HashMap;
#[cfg(feature = "tui")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "tui")]
use std::sync::{Arc, RwLock};

use chrono::{DateTime, NaiveDate, Utc};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

use data_feed::account::resolve_capital;
use data_feed::alpaca_feed::{AlpacaFeed, BarEvent};
use data_feed::broker::{AlpacaBroker, Broker, SimulatedBroker};
use data_feed::config_loader::load_config;
use data_feed::config_watcher::{try_build_engine, ConfigWatcher};
use data_feed::live_session::{LiveSession, TradeWithScores};
use data_feed::market_state::MarketStateBuilder;
use data_feed::cross_tracker::CrossTracker;
use data_feed::session_clock::{eastern_date, eastern_minutes, is_regular_hours, parse_hm};
use data_feed::state_writer::{
    delete_engine_state, upsert_engine_state, write_entry_block_event, BlockEvent, BlockKind,
    EngineStateRow, PositionSnapshot,
};
use data_feed::trade_writer::TradeWriter;
use types::action::ExitReason;
use types::config::StrategyConfig;
use types::tick_result::TickEvent;

#[cfg(feature = "tui")]
use data_feed::tui::{DashboardState, PositionDisplay, TickerState, TradeLogEntry};

/// calendar days of 1-minute history to fetch at startup. 8 calendar days
/// covers ≥5 trading days even across a long weekend, enough to fill the
/// hourly window (21+ hourly candles) for the hourly indicators.
const WARMUP_LOOKBACK_DAYS: i64 = 8;
/// no bar for this long during regular hours ⇒ the feed is considered stale.
const FEED_STALE_AFTER_SECS: i64 = 180;
/// a near-miss diagnostic is persisted at most this often per ticker.
/// index symbol streamed alongside the traded tickers to fill `MarketState.cross`.
const CROSS_INDEX_SYMBOL: &str = "SPY";
const NEAR_MISS_THROTTLE_SECS: i64 = 300;
/// a repeated gate reason is re-persisted at most this often per ticker.
const GATE_REPEAT_SECS: i64 = 900;

/// per-process runtime state shared by the loop arms.
struct Runtime {
    sessions: HashMap<String, LiveSession>,
    /// index + peer session state for `MarketState.cross` (same definition as the replay).
    cross: CrossTracker,
    state_builders: HashMap<String, MarketStateBuilder>,
    last_bar_at: HashMap<String, DateTime<Utc>>,
    last_prices: HashMap<String, f64>,
    feed_stale: HashMap<String, bool>,
    /// last persisted gate reason and when, per ticker (dedupe).
    last_gate: HashMap<String, (String, DateTime<Utc>)>,
    /// last persisted near-miss time, per ticker (throttle).
    last_near_miss: HashMap<String, DateTime<Utc>>,
    /// config currently driving new engines.
    current_config: StrategyConfig,
    current_config_id: i64,
    /// config waiting for in-position tickers to go flat.
    pending: Option<(i64, StrategyConfig)>,
    capital: f64,
    daily_pnl: f64,
    daily_pnl_date: Option<NaiveDate>,
    tick_count: u64,
    process_started_at: DateTime<Utc>,
    broker_mode: String,
}

impl Runtime {
    fn max_concurrent(&self) -> usize {
        self.current_config.session.max_concurrent_positions as usize
    }

    fn open_position_count(&self) -> usize {
        self.sessions.values().filter(|s| s.has_position()).count()
    }

    /// roll the process-wide daily P&L on the first bar of a new eastern day.
    fn roll_daily_pnl(&mut self, ts: DateTime<Utc>) {
        let d = eastern_date(ts);
        if self.daily_pnl_date != Some(d) {
            if self.daily_pnl_date.is_some() {
                info!(date = %d, prev_daily_pnl = format!("{:.2}", self.daily_pnl), "new trading day");
            }
            self.daily_pnl_date = Some(d);
            self.daily_pnl = 0.0;
        }
    }

    fn state_row(&self, ticker: &str) -> Option<EngineStateRow> {
        let session = self.sessions.get(ticker)?;
        let last = session.last_result();
        let position = session.current_position().map(|p| PositionSnapshot {
            direction: p.direction,
            entry_price: p.entry_price,
            size: p.size,
            unrealized_pnl: p.unrealized_pnl,
            unrealized_pnl_pct: p.unrealized_pnl_pct,
            hold_ms: p.hold_duration_ms,
            opened_at: p.entry_time,
            entry_reason: session.entry_reason().map(|s| s.to_string()),
        });
        Some(EngineStateRow {
            ticker: ticker.to_string(),
            last_bar_at: self.last_bar_at.get(ticker).copied(),
            last_price: self.last_prices.get(ticker).copied(),
            scores: session.last_scores().clone(),
            position,
            daily_pnl: self.daily_pnl,
            ticker_realized_pnl: session.engine().cumulative_realized_pnl(),
            loss_breaker_active: session.engine().is_daily_loss_breaker_active(),
            entry_blocked_by: last.and_then(|r| r.entry_blocked_by.clone()),
            near_miss: last.and_then(|r| r.near_miss.clone()),
            feed_stale: self.feed_stale.get(ticker).copied().unwrap_or(false),
            config_version_id: session.config_version_id(),
            pending_config_version_id: self.pending.as_ref().map(|(id, _)| *id),
            process_started_at: self.process_started_at,
            broker_mode: self.broker_mode.clone(),
        })
    }
}

fn shutdown_summary(rt: &Runtime) {
    info!(
        total_ticks = rt.tick_count,
        daily_pnl = format!("{:.2}", rt.daily_pnl),
        open_positions = rt.open_position_count(),
        "shutdown complete"
    );
}

/// close the position at the broker and record the trade. `price` is the
/// engine-side exit price (last known bar close) used when the broker has
/// no better number.
async fn close_and_record(
    broker: &dyn Broker,
    trade_writer: &TradeWriter,
    session: &mut LiveSession,
    ticker: &str,
    price: f64,
    reason: ExitReason,
    pool: &sqlx::PgPool,
) {
    let broker_exit = match broker.close_position(ticker).await {
        Ok(fill) => {
            info!(ticker, fill_price = fill.fill_price, ?reason, "broker position closed");
            Some(fill.fill_price)
        }
        Err(e) => {
            error!(ticker, error = %e, ?reason, "broker close FAILED — check the account manually");
            None
        }
    };
    let engine_price = if price > 0.0 { price } else { broker_exit.unwrap_or(0.0) };
    match session.force_close(engine_price, Utc::now(), reason) {
        Some(tws) => record_trade(trade_writer, &tws, broker_exit, pool).await,
        None => warn!(ticker, "force_close produced no trade (engine had no position)"),
    }
}

async fn record_trade(
    trade_writer: &TradeWriter,
    tws: &TradeWithScores,
    broker_exit_price: Option<f64>,
    _pool: &sqlx::PgPool,
) {
    info!(
        ticker = %tws.trade.ticker,
        pnl = format!("{:.2}", tws.trade.pnl),
        pnl_pct = format!("{:.2}%", tws.trade.pnl_pct * 100.0),
        exit_reason = ?tws.trade.exit_reason,
        entry_reason = %tws.entry_reason,
        hold_ms = tws.trade.hold_duration_ms,
        config_version = tws.config_version_id,
        "trade completed"
    );
    match trade_writer.write_trade(tws, broker_exit_price).await {
        Ok(trade_id) => info!(trade_id, "trade written to database"),
        Err(e) => error!(error = %e, "failed to write trade to database"),
    }
}

/// flatten every open position (shutdown / emergency).
async fn flatten_all(
    rt: &mut Runtime,
    broker: &dyn Broker,
    trade_writer: &TradeWriter,
    pool: &sqlx::PgPool,
    reason: ExitReason,
) {
    let tickers: Vec<String> = rt
        .sessions
        .iter()
        .filter(|(_, s)| s.has_position())
        .map(|(t, _)| t.clone())
        .collect();
    for ticker in tickers {
        let price = rt.last_prices.get(&ticker).copied().unwrap_or(0.0);
        warn!(ticker = %ticker, ?reason, "force-closing open position");
        if let Some(session) = rt.sessions.get_mut(&ticker) {
            close_and_record(broker, trade_writer, session, &ticker, price, reason.clone(), pool).await;
            rt.daily_pnl += session
                .completed_trades()
                .last()
                .map(|t| t.pnl)
                .unwrap_or(0.0);
        }
    }
}

/// swap a flat ticker's engine to the pending config. returns true if swapped.
fn apply_pending_to(rt: &mut Runtime, ticker: &str) -> bool {
    let Some((pending_id, pending_cfg)) = rt.pending.as_ref() else {
        return false;
    };
    let pending_id = *pending_id;
    if !pending_cfg.tickers.iter().any(|t| t == ticker) {
        // ticker removed by the new config: drop it now that it is flat
        info!(ticker, version_id = pending_id, "ticker not in new config, dropping session");
        rt.sessions.remove(ticker);
        rt.state_builders.remove(ticker);
        return true;
    }
    match try_build_engine(pending_cfg, ticker, rt.capital) {
        Some(engine) => {
            info!(ticker, version_id = pending_id, "engine rebuilt for new config");
            rt.sessions
                .insert(ticker.to_string(), LiveSession::new(engine, pending_id));
            true
        }
        None => {
            warn!(ticker, version_id = pending_id, "failed to rebuild engine, keeping old");
            false
        }
    }
}

/// apply the pending config to every flat ticker; clear it when nothing is left waiting.
fn apply_pending_where_flat(rt: &mut Runtime) {
    let Some((pending_id, _)) = rt.pending.as_ref() else {
        return;
    };
    let pending_id = *pending_id;
    let tickers: Vec<String> = rt.sessions.keys().cloned().collect();
    for t in tickers {
        let needs_swap = rt
            .sessions
            .get(&t)
            .map(|s| s.config_version_id() != pending_id && !s.has_position())
            .unwrap_or(false);
        if needs_swap {
            apply_pending_to(rt, &t);
        }
    }
    let still_waiting = rt
        .sessions
        .values()
        .any(|s| s.config_version_id() != pending_id);
    if !still_waiting {
        let (id, cfg) = rt.pending.take().expect("pending checked above");
        info!(version_id = id, "config swap complete on all tickers");
        rt.current_config = cfg;
        rt.current_config_id = id;
    }
}

/// persist gate / near-miss diagnostics with dedupe and throttling.
async fn record_block_events(
    rt: &mut Runtime,
    pool: &sqlx::PgPool,
    ticker: &str,
    ts: DateTime<Utc>,
    result: &types::tick_result::TickResult,
    last_price: f64,
    config_version_id: i64,
) {
    if let Some(reason) = &result.entry_blocked_by {
        let repeat = rt
            .last_gate
            .get(ticker)
            .map(|(r, at)| r == reason && (ts - *at).num_seconds() < GATE_REPEAT_SECS)
            .unwrap_or(false);
        if !repeat {
            let ev = BlockEvent {
                ticker,
                ts,
                kind: BlockKind::Gate,
                reason,
                scores: &result.scores,
                last_price,
                config_version_id,
            };
            if let Err(e) = write_entry_block_event(pool, &ev).await {
                warn!(error = %e, "failed to write entry block event");
            }
            rt.last_gate
                .insert(ticker.to_string(), (reason.clone(), ts));
        }
    } else if let Some(nm) = &result.near_miss {
        let throttled = rt
            .last_near_miss
            .get(ticker)
            .map(|at| (ts - *at).num_seconds() < NEAR_MISS_THROTTLE_SECS)
            .unwrap_or(false);
        if !throttled {
            debug!(ticker, near_miss = %nm, "entry near-miss");
            let ev = BlockEvent {
                ticker,
                ts,
                kind: BlockKind::NearMiss,
                reason: nm,
                scores: &result.scores,
                last_price,
                config_version_id,
            };
            if let Err(e) = write_entry_block_event(pool, &ev).await {
                warn!(error = %e, "failed to write near-miss event");
            }
            rt.last_near_miss.insert(ticker.to_string(), ts);
        }
    }
}

async fn persist_state(rt: &Runtime, pool: &sqlx::PgPool, ticker: &str) {
    if let Some(row) = rt.state_row(ticker) {
        if let Err(e) = upsert_engine_state(pool, &row).await {
            warn!(ticker, error = %e, "failed to upsert engine_state");
        }
    }
}

#[tokio::main]
async fn main() {
    // 0. load .env file (ok if missing)
    let _ = dotenvy::dotenv();

    // 1. init structured logging (console + file layers)
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    std::fs::create_dir_all(&log_dir).expect("failed to create log directory");

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}/paper_trader_{}.log", log_dir, today))
        .expect("failed to open log file");

    let (non_blocking, _file_guard) = tracing_appender::non_blocking(log_file);
    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(EnvFilter::new("info"));

    // console layer: JSON stdout (headless) or stderr warn+ (TUI)
    #[cfg(not(feature = "tui"))]
    let console_layer = {
        let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        tracing_subscriber::fmt::layer().json().with_filter(filter)
    };
    #[cfg(feature = "tui")]
    let console_layer = {
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(filter)
    };

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    info!("galactic trading firm — paper trading engine starting");

    // 2. load environment
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let api_key = std::env::var("APCA_API_KEY_ID").unwrap_or_default();
    let api_secret = std::env::var("APCA_API_SECRET_KEY").unwrap_or_default();
    let broker_mode = std::env::var("BROKER_MODE").unwrap_or_else(|_| "simulated".to_string());
    let demo_mode = std::env::args().any(|a| a == "--demo");

    // credentials are mandatory unless the operator explicitly asked for the synthetic feed
    if !demo_mode && (api_key.is_empty() || api_secret.is_empty()) {
        error!(
            "APCA_API_KEY_ID / APCA_API_SECRET_KEY are not set. refusing to start: \
             the synthetic feed is only available with the explicit --demo flag"
        );
        std::process::exit(1);
    }
    if demo_mode && broker_mode == "alpaca_paper" {
        error!("--demo cannot be combined with BROKER_MODE=alpaca_paper (would place real paper orders on fake prices)");
        std::process::exit(1);
    }
    let trade_source = if demo_mode { "demo" } else { "paper" };

    // 3. connect to postgres
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");
    info!("connected to database");

    // 4. load promoted config (row id is what trades/hot-reload are keyed on)
    let (config_version_id, strategy_config) = match load_config(&pool).await {
        Ok((id, config)) => {
            info!(
                config_version_id = id,
                blob_config_id = config.config_id,
                indicators = config.indicators.len(),
                actions = config.actions.len(),
                tickers = ?config.tickers,
                force_exit_by = %config.session.force_exit_by,
                no_new_entries_after = %config.session.no_new_entries_after,
                avoid_first_minutes = config.session.avoid_first_minutes,
                max_position_pct = ?config.session.max_position_pct,
                "loaded promoted config"
            );
            (id, config)
        }
        Err(e) => {
            error!(error = %e, "failed to load config");
            std::process::exit(1);
        }
    };
    if config_version_id != strategy_config.config_id {
        warn!(
            row_id = config_version_id,
            blob_config_id = strategy_config.config_id,
            "config blob's config_id differs from its row id — using the row id for attribution"
        );
    }

    // 5. resolve capital
    let env_capital: f64 = std::env::var("INITIAL_CAPITAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000.0);
    let capital = resolve_capital(&broker_mode, &api_key, &api_secret, env_capital).await;
    info!(
        capital,
        env_capital,
        "capital resolved (each ticker engine sizes against this; max_concurrent_positions bounds exposure)"
    );

    // 6. build engine per ticker
    let mut sessions: HashMap<String, LiveSession> = HashMap::new();
    let mut state_builders: HashMap<String, MarketStateBuilder> = HashMap::new();

    for ticker in &strategy_config.tickers {
        match try_build_engine(&strategy_config, ticker, capital) {
            Some(engine) => {
                info!(ticker = %ticker, "engine built");
                sessions.insert(ticker.clone(), LiveSession::new(engine, config_version_id));
                state_builders.insert(ticker.clone(), MarketStateBuilder::new(200));
            }
            None => {
                error!(ticker = %ticker, "failed to build engine, skipping ticker");
            }
        }
    }

    if sessions.is_empty() {
        error!("no engines could be built, exiting");
        std::process::exit(1);
    }

    // 7. create broker
    let broker: Box<dyn Broker> = match broker_mode.as_str() {
        "alpaca_paper" => {
            let b = AlpacaBroker::new(api_key.clone(), api_secret.clone())
                .expect("failed to create alpaca broker");
            info!("using alpaca paper broker");
            Box::new(b)
        }
        "simulated" => {
            info!("using simulated broker (5 bps slippage)");
            Box::new(SimulatedBroker::new(5.0))
        }
        other => {
            error!(broker_mode = other, "unknown BROKER_MODE (expected 'simulated' or 'alpaca_paper')");
            std::process::exit(1);
        }
    };

    // 7b. reconcile: the engine starts flat; anything the broker already holds is an orphan
    match broker.open_positions().await {
        Ok(open) if !open.is_empty() => {
            for p in &open {
                error!(
                    ticker = %p.ticker,
                    direction = ?p.direction,
                    quantity = p.quantity,
                    entry_price = p.entry_price,
                    "BROKER HOLDS A POSITION THE ENGINE DOES NOT KNOW ABOUT — close it manually or it will be ignored"
                );
            }
        }
        Ok(_) => info!("broker reconciliation: no pre-existing positions"),
        Err(e) => warn!(error = %e, "could not list broker positions for reconciliation"),
    }

    // 8. trade writer + config watcher (both keyed on the row id)
    let trade_writer = TradeWriter::new(pool.clone(), trade_source);
    let mut config_watcher = ConfigWatcher::new(pool.clone(), config_version_id);

    // 9. data feed and channel
    let (bar_tx, mut bar_rx) = mpsc::channel::<BarEvent>(1000);
    let tickers: Vec<String> = strategy_config.tickers.clone();

    // 10. warm the candle windows with history, then stream
    let mut cross = CrossTracker::new(CROSS_INDEX_SYMBOL);
    if !demo_mode {
        let feed = AlpacaFeed::new(api_key.clone(), api_secret.clone(), tickers.clone());
        // the index feeds MarketState.cross only; seed it so a mid-session restart knows
        // today's session open (the tracker filters by eastern date itself).
        match feed.fetch_historical_bars(CROSS_INDEX_SYMBOL, 1).await {
            Ok(candles) => {
                cross.seed(CROSS_INDEX_SYMBOL, &candles);
                info!(symbol = CROSS_INDEX_SYMBOL, bars = candles.len(), "cross-context index seeded");
            }
            Err(e) => warn!(symbol = CROSS_INDEX_SYMBOL, error = %e, "cross-context index seed failed — SPY-conditioned windows stay closed until live SPY bars arrive today"),
        }
        for ticker in &tickers {
            info!(ticker = %ticker, days = WARMUP_LOOKBACK_DAYS, "fetching historical bars for warmup");
            match feed.fetch_historical_bars(ticker, WARMUP_LOOKBACK_DAYS).await {
                Ok(candles) => {
                    let count = candles.len();
                    cross.seed(ticker, &candles);
                    if let Some(builder) = state_builders.get_mut(ticker) {
                        builder.seed(candles);
                        info!(
                            ticker = %ticker,
                            one_minute_bars = count,
                            windows = ?builder.window_sizes(),
                            "warmup complete"
                        );
                    }
                    if count < 3 * 390 {
                        warn!(
                            ticker = %ticker,
                            one_minute_bars = count,
                            "fewer than 3 trading days of history — hourly indicators will be partially warm"
                        );
                    }
                }
                Err(e) => {
                    error!(ticker = %ticker, error = %e, "failed to fetch historical bars — starting cold; hourly indicators will be WRONG for hours");
                }
            }
        }

        let mut stream_symbols = tickers.clone();
        stream_symbols.push(CROSS_INDEX_SYMBOL.to_string());
        let feed = AlpacaFeed::new(api_key, api_secret, stream_symbols);
        tokio::spawn(async move {
            if let Err(e) = feed.stream_bars(bar_tx).await {
                error!(error = %e, "data feed stream failed");
            }
        });
    } else {
        warn!("--demo: starting SYNTHETIC random-walk feed; trades are written with source='demo'");
        let demo_tickers = tickers.clone();
        tokio::spawn(async move {
            let base_prices: HashMap<&str, f64> = [
                ("SPY", 450.0), ("QQQ", 380.0), ("AAPL", 175.0),
                ("NVDA", 800.0), ("MSFT", 420.0), ("AMZN", 200.0),
            ]
            .into_iter()
            .collect();

            let mut prices: HashMap<String, f64> = demo_tickers
                .iter()
                .map(|t| (t.clone(), base_prices.get(t.as_str()).copied().unwrap_or(100.0)))
                .collect();

            let mut rng_state: u64 = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;

            loop {
                for ticker in &demo_tickers {
                    let Some(price) = prices.get_mut(ticker) else { continue };
                    rng_state ^= rng_state << 13;
                    rng_state ^= rng_state >> 7;
                    rng_state ^= rng_state << 17;
                    let norm = ((rng_state % 10000) as f64 / 10000.0 - 0.5) * 2.0;
                    let volatility = *price * 0.002;
                    let change = norm * volatility;
                    *price = (*price + change).max(1.0);
                    let close = *price;
                    let open = close - change * 0.3;
                    let high = close.max(open).max(close + volatility * 0.5);
                    let low = close.min(open).min(close - volatility * 0.5);
                    let candle = types::market::Candle {
                        timestamp: chrono::Utc::now(),
                        open, high, low, close,
                        volume: 10000.0 + (rng_state % 50000) as f64,
                    };
                    if bar_tx.send(BarEvent { symbol: ticker.clone(), candle }).await.is_err() {
                        return;
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    // 11. timers
    let mut config_reload_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    config_reload_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    heartbeat_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut rt = Runtime {
        sessions,
        cross,
        state_builders,
        last_bar_at: HashMap::new(),
        last_prices: HashMap::new(),
        feed_stale: HashMap::new(),
        last_gate: HashMap::new(),
        last_near_miss: HashMap::new(),
        current_config: strategy_config.clone(),
        current_config_id: config_version_id,
        pending: None,
        capital,
        daily_pnl: 0.0,
        daily_pnl_date: None,
        tick_count: 0,
        process_started_at: Utc::now(),
        broker_mode: broker_mode.clone(),
    };

    // initial heartbeat rows so observers see the process immediately
    let initial: Vec<String> = rt.sessions.keys().cloned().collect();
    for t in &initial {
        persist_state(&rt, &pool, t).await;
    }

    // 11b. launch TUI if feature is enabled
    #[cfg(feature = "tui")]
    let tui_shutdown = Arc::new(AtomicBool::new(false));
    #[cfg(feature = "tui")]
    let dashboard_state = Arc::new(RwLock::new(DashboardState::new(config_version_id)));
    #[cfg(feature = "tui")]
    {
        dashboard_state
            .write()
            .unwrap()
            .set_strategy_config(strategy_config.clone());

        let state = Arc::clone(&dashboard_state);
        let shutdown = Arc::clone(&tui_shutdown);
        std::thread::spawn(move || {
            if let Err(e) = data_feed::tui::run_tui(state, shutdown) {
                eprintln!("TUI error: {e}");
            }
        });
    }

    // register SIGTERM handler for graceful container shutdown
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");

    info!("entering main trading loop");

    // 12. main select! loop
    loop {
        #[cfg(feature = "tui")]
        if tui_shutdown.load(Ordering::Relaxed) {
            info!("TUI requested shutdown");
            flatten_all(&mut rt, broker.as_ref(), &trade_writer, &pool, ExitReason::ManualOverride).await;
            shutdown_summary(&rt);
            break;
        }

        tokio::select! {
            // ── incoming bar from data feed ──
            Some(bar_event) = bar_rx.recv() => {
                let ticker = bar_event.symbol.clone();
                let bar_ts = bar_event.candle.timestamp;

                // only regular-hours bars reach the engine (demo bars are exempt)
                if !demo_mode && !is_regular_hours(bar_ts) {
                    debug!(ticker = %ticker, ts = %bar_ts, "ignoring extended-hours bar");
                    continue;
                }

                // every regular-hours bar (index included) feeds the cross-context tracker
                rt.cross.on_bar(&ticker, &bar_event.candle);
                if !rt.sessions.contains_key(&ticker) {
                    debug!(symbol = %ticker, "index/peer bar recorded (not traded)");
                    continue;
                }

                rt.tick_count += 1;
                rt.last_bar_at.insert(ticker.clone(), bar_ts);
                rt.last_prices.insert(ticker.clone(), bar_event.candle.close);
                if rt.feed_stale.remove(&ticker).unwrap_or(false) {
                    info!(ticker = %ticker, "feed resumed");
                }
                rt.roll_daily_pnl(bar_ts);
                broker.set_last_price(bar_event.candle.close);

                #[cfg(feature = "tui")]
                let candle_for_tui = bar_event.candle.clone();

                // build market state from candle
                let mut market = match rt.state_builders.get_mut(&ticker) {
                    Some(builder) => {
                        let bid = bar_event.candle.close - 0.01;
                        let ask = bar_event.candle.close + 0.01;
                        builder.on_bar(bar_event.candle, bid, ask)
                    }
                    None => {
                        debug!(ticker = %ticker, "no state builder for ticker (not traded), skipping");
                        continue;
                    }
                };

                // cross-ticker context (index + peers), same definition as the replay
                let traded: Vec<String> = rt.sessions.keys().cloned().collect();
                market.cross = rt.cross.context_for(&ticker, &traded, bar_ts);

                // cross-ticker capacity: still tick (exits, clocks, diagnostics), but block entries
                let at_capacity = rt.open_position_count() >= rt.max_concurrent();
                let Some(session) = rt.sessions.get_mut(&ticker) else { continue };
                if at_capacity && !session.has_position() {
                    market.entries_blocked = true;
                }

                let (result, trade_with_scores) = session.on_tick(&mut market);
                let session_cfg_id = session.config_version_id();

                if let TickEvent::PositionOpened = &result.event {
                    let Some(pos) = session.current_position() else { continue };
                    // engine sizes in whole shares already; the broker gets exactly that
                    let shares = pos.size.floor();
                    let direction = pos.direction;
                    info!(
                        ticker = %ticker,
                        composite = result.scores.composite,
                        price = market.last_price,
                        shares,
                        entry_reason = %result.entry_reason,
                        "position opened"
                    );
                    if shares < 1.0 {
                        error!(ticker = %ticker, size = pos.size, "engine opened a sub-share position, undoing");
                        session.undo_open();
                    } else {
                        match broker.submit_order(&ticker, direction, shares).await {
                            Ok(fill) => {
                                info!(
                                    ticker = %ticker,
                                    fill_price = fill.fill_price,
                                    quantity = fill.quantity,
                                    engine_price = market.last_price,
                                    "broker order filled"
                                );
                                session.set_broker_entry_price(fill.fill_price);
                            }
                            Err(e) => {
                                error!(ticker = %ticker, error = %e, "broker order failed, undoing position");
                                session.undo_open();
                            }
                        }
                    }
                }

                // update TUI dashboard state
                #[cfg(feature = "tui")]
                {
                    let mut dash = dashboard_state.write().unwrap();
                    dash.total_ticks = rt.tick_count;
                    dash.daily_pnl = rt.daily_pnl;

                    let pos_display = session.current_position().map(|pos| PositionDisplay {
                        direction: pos.direction,
                        entry_price: pos.entry_price,
                        unrealized_pnl: pos.unrealized_pnl,
                        unrealized_pnl_pct: pos.unrealized_pnl_pct,
                        hold_duration_ms: pos.hold_duration_ms,
                    });

                    dash.ensure_ticker_order(&ticker);
                    let ts = dash.tickers.entry(ticker.clone()).or_insert_with(|| TickerState {
                        ticker: ticker.clone(),
                        last_price: 0.0,
                        composite_score: 0.0,
                        one_minute_score: None,
                        five_minute_score: None,
                        one_hour_score: None,
                        position: None,
                        price_history: Vec::new(),
                        candle_history: Vec::new(),
                    });
                    ts.last_price = market.last_price;
                    ts.composite_score = result.scores.composite;
                    ts.one_minute_score = result.scores.one_minute;
                    ts.five_minute_score = result.scores.five_minute;
                    ts.one_hour_score = result.scores.one_hour;
                    ts.position = pos_display;
                    ts.price_history.push(market.last_price);
                    if ts.price_history.len() > 60 {
                        ts.price_history.remove(0);
                    }
                    ts.candle_history.push(candle_for_tui.clone());
                    if ts.candle_history.len() > 60 {
                        ts.candle_history.remove(0);
                    }
                }

                if let Some(tws) = trade_with_scores {
                    let broker_exit = match broker.close_position(&tws.trade.ticker).await {
                        Ok(fill) => {
                            info!(ticker = %tws.trade.ticker, fill_price = fill.fill_price, engine_price = tws.trade.exit_price, "broker position closed");
                            Some(fill.fill_price)
                        }
                        Err(e) => {
                            error!(ticker = %tws.trade.ticker, error = %e, "broker close FAILED (engine already closed) — check the account");
                            None
                        }
                    };
                    rt.daily_pnl += tws.trade.pnl;

                    #[cfg(feature = "tui")]
                    {
                        let mut dash = dashboard_state.write().unwrap();
                        dash.daily_pnl = rt.daily_pnl;
                        dash.add_trade(TradeLogEntry {
                            ticker: tws.trade.ticker.clone(),
                            direction: format!("{:?}", tws.trade.direction),
                            pnl: tws.trade.pnl,
                            pnl_pct: tws.trade.pnl_pct,
                            exit_reason: format!("{:?}", tws.trade.exit_reason),
                            time: tws.trade.exit_time,
                        });
                    }

                    record_trade(&trade_writer, &tws, broker_exit, &pool).await;

                    // this ticker is flat now — if a config is waiting, swap it in
                    if rt.pending.is_some() {
                        apply_pending_where_flat(&mut rt);
                    }
                }

                // observability: state row + gate/near-miss events
                record_block_events(&mut rt, &pool, &ticker, bar_ts, &result, market.last_price, session_cfg_id).await;
                persist_state(&rt, &pool, &ticker).await;

                // periodic status log
                if rt.tick_count.is_multiple_of(60) {
                    let positions: Vec<&str> = rt
                        .sessions
                        .iter()
                        .filter(|(_, s)| s.has_position())
                        .map(|(t, _)| t.as_str())
                        .collect();
                    info!(
                        ticks = rt.tick_count,
                        daily_pnl = format!("{:.2}", rt.daily_pnl),
                        open_positions = ?positions,
                        pending_config = ?rt.pending.as_ref().map(|(id, _)| *id),
                        "status"
                    );
                }
            }

            // ── heartbeat: staleness, missed-close safety net, state rows ──
            _ = heartbeat_interval.tick() => {
                let now = Utc::now();
                let market_open_now = is_regular_hours(now);

                // feed staleness per traded ticker
                if market_open_now && !demo_mode {
                    let tickers: Vec<String> = rt.sessions.keys().cloned().collect();
                    for t in tickers {
                        let age = rt.last_bar_at.get(&t).map(|ts| (now - *ts).num_seconds());
                        let stale = match age {
                            Some(a) => a > FEED_STALE_AFTER_SECS,
                            // never seen a bar: only stale once we're well past the open
                            None => eastern_minutes(now) > 9 * 60 + 30 + 5,
                        };
                        let was = rt.feed_stale.get(&t).copied().unwrap_or(false);
                        if stale && !was {
                            warn!(ticker = %t, last_bar_age_s = ?age, "FEED STALE — no bar during market hours");
                        }
                        rt.feed_stale.insert(t, stale);
                    }
                }

                // missed-close safety net: a halted/silent ticker gets no bars, so the
                // engine never sees the force_exit_by tick. close it from the clock instead.
                if let Some(cutoff) = parse_hm(&rt.current_config.session.force_exit_by) {
                    if eastern_minutes(now) >= cutoff + 2 && rt.open_position_count() > 0 {
                        warn!("positions still open past force_exit_by — flattening from the clock");
                        flatten_all(&mut rt, broker.as_ref(), &trade_writer, &pool, ExitReason::SessionClose).await;
                        if rt.pending.is_some() {
                            apply_pending_where_flat(&mut rt);
                        }
                    }
                }

                let tickers: Vec<String> = rt.sessions.keys().cloned().collect();
                for t in &tickers {
                    persist_state(&rt, &pool, t).await;
                }
            }

            // ── config reload timer ──
            _ = config_reload_interval.tick() => {
                match config_watcher.check_for_update().await {
                    Ok(Some((version_id, new_config))) => {
                        let open: Vec<&String> = rt.sessions.iter().filter(|(_, s)| s.has_position()).map(|(t, _)| t).collect();
                        info!(version_id, open_positions = ?open, "new config detected; applying to flat tickers now, others when they close");

                        // tickers added by the new config need a websocket subscription we don't have
                        for t in &new_config.tickers {
                            if !rt.sessions.contains_key(t) {
                                warn!(ticker = %t, version_id, "new config adds a ticker — requires a restart to subscribe; ignoring for now");
                            }
                        }
                        // tickers removed by the new config: state rows go away when they are dropped
                        for t in rt.sessions.keys() {
                            if !new_config.tickers.contains(t) {
                                let _ = delete_engine_state(&pool, t).await;
                            }
                        }

                        rt.pending = Some((version_id, new_config.clone()));
                        config_watcher.acknowledge(version_id);
                        apply_pending_where_flat(&mut rt);

                        #[cfg(feature = "tui")]
                        {
                            let mut dash = dashboard_state.write().unwrap();
                            dash.config_version = version_id;
                            dash.set_strategy_config(new_config);
                        }
                        let tickers: Vec<String> = rt.sessions.keys().cloned().collect();
                        for t in &tickers {
                            persist_state(&rt, &pool, t).await;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        warn!(error = %e, "config check failed, continuing with current config");
                    }
                }
            }

            // ── graceful shutdown on ctrl-c ──
            _ = tokio::signal::ctrl_c() => {
                info!("ctrl-c received, shutting down");
                #[cfg(feature = "tui")]
                tui_shutdown.store(true, Ordering::Relaxed);
                flatten_all(&mut rt, broker.as_ref(), &trade_writer, &pool, ExitReason::ManualOverride).await;
                shutdown_summary(&rt);
                break;
            }

            // ── graceful shutdown on SIGTERM (docker stop / kill -TERM) ──
            _ = sigterm.recv() => {
                info!("SIGTERM received, shutting down");
                #[cfg(feature = "tui")]
                tui_shutdown.store(true, Ordering::Relaxed);
                flatten_all(&mut rt, broker.as_ref(), &trade_writer, &pool, ExitReason::ManualOverride).await;
                shutdown_summary(&rt);
                break;
            }
        }
    }

    // final state rows so observers see the process is gone cleanly
    let tickers: Vec<String> = rt.sessions.keys().cloned().collect();
    for t in &tickers {
        persist_state(&rt, &pool, t).await;
    }
}
