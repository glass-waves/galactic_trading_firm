//! paper_trader — live market data → trading engines (one per book × ticker) →
//! (simulated | alpaca paper) broker → postgres.
//!
//! operational guarantees this binary is responsible for (see docs/paper_trading_plan_2026-09.md
//! and docs/dev/books_trader_notes.md):
//! - never trades on a synthetic feed unless `--demo` is passed explicitly
//! - only regular-trading-hours bars reach the engines
//! - every open position of every book is closed at its broker (and recorded) on shutdown,
//!   on config swap, and if the session close is missed for lack of bars
//! - config hot-reload is deferred per ticker until that ticker is flat
//! - live state (scores, positions, gates, heartbeat) is persisted to
//!   `engine_state` / `entry_block_events` / `book_sessions` so an outside observer can see it
//! - the primary book is the only book that can hold an Alpaca broker; shadows are simulated
//!
//! `--replay-bars DIR --replay-date D --replay-out FILE` runs the same loop on cached bars
//! with simulated brokers and no database writes (see `replay_feed`).

use std::collections::{BTreeSet, HashMap};
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
use data_feed::book::{
    apply_limits, load_books, Book, BookPlan, BookSpec, Limits, ReplayLog, Sink, TickerShared,
    SHADOW_SLIPPAGE_BPS,
};
use data_feed::broker::{AlpacaBroker, Broker, SimulatedBroker};
use data_feed::config_loader::{load_config, load_config_by_id};
use data_feed::config_watcher::ConfigWatcher;
use data_feed::cross_tracker::CrossTracker;
use data_feed::market_state::MarketStateBuilder;
use data_feed::replay_feed::{load_replay_bars, write_replay_trades, REPLAY_WARMUP_DAYS};
use data_feed::session_clock::{eastern_minutes, is_regular_hours, parse_hm};
use data_feed::state_writer::delete_unhosted_engine_state;
use data_feed::trade_writer::TradeWriter;
use types::action::ExitReason;

#[cfg(feature = "tui")]
use data_feed::tui::{DashboardState, PositionDisplay, TickerState, TradeLogEntry};

/// calendar days of 1-minute history to fetch at startup. 8 calendar days
/// covers ≥5 trading days even across a long weekend, enough to fill the
/// hourly window (21+ hourly candles) for the hourly indicators.
const WARMUP_LOOKBACK_DAYS: i64 = 8;
/// no bar for this long during regular hours ⇒ the feed is considered stale.
const FEED_STALE_AFTER_SECS: i64 = 180;
/// index symbol streamed alongside the traded tickers to fill `MarketState.cross`.
const CROSS_INDEX_SYMBOL: &str = "SPY";

/// how this process is fed and where its output goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// alpaca websocket, real or simulated broker for the primary, postgres writes.
    Live,
    /// synthetic random-walk feed, simulated brokers, postgres writes with source='demo'.
    Demo,
    /// cached bars, simulated brokers, no database writes, exits when the bars run out.
    Replay,
}

/// per-process shared state: everything that is per-symbol rather than per-strategy.
struct Runtime {
    /// index + peer session state for `MarketState.cross` (same definition as the replay).
    cross: CrossTracker,
    /// candle windows + session VWAP per hosted ticker, shared by every book.
    state_builders: HashMap<String, MarketStateBuilder>,
    last_bar_at: HashMap<String, DateTime<Utc>>,
    last_prices: HashMap<String, f64>,
    feed_stale: HashMap<String, bool>,
    /// newest bar timestamp seen on any symbol (the clock in replay mode).
    last_any_bar_ts: Option<DateTime<Utc>>,
    tick_count: u64,
    mode: Mode,
    /// the primary first, then the shadows.
    books: Vec<Book>,
}

impl Runtime {
    /// wall clock live; the bar clock in replay (every `Utc::now()` stamp uses the bar time).
    fn now(&self) -> DateTime<Utc> {
        match self.mode {
            Mode::Replay => self.last_any_bar_ts.unwrap_or_else(Utc::now),
            _ => Utc::now(),
        }
    }

    fn primary(&self) -> &Book {
        &self.books[0]
    }

    /// does any book trade this ticker?
    fn is_hosted(&self, ticker: &str) -> bool {
        self.books.iter().any(|b| b.trades(ticker))
    }

    /// every ticker some book trades (sorted).
    fn hosted_tickers(&self) -> Vec<String> {
        let set: BTreeSet<String> =
            self.books.iter().flat_map(|b| b.tickers().iter().cloned()).collect();
        set.into_iter().collect()
    }

    fn shared_for(&self, ticker: &str) -> TickerShared {
        TickerShared {
            last_bar_at: self.last_bar_at.get(ticker).copied(),
            last_price: self.last_prices.get(ticker).copied(),
            feed_stale: self.feed_stale.get(ticker).copied().unwrap_or(false),
        }
    }

    /// drop shared builders for tickers no book trades any more (after a config swap
    /// removed the ticker from its last book). the cross tracker keeps its own state.
    fn prune_unhosted_builders(&mut self) {
        let stale: Vec<String> = self
            .state_builders
            .keys()
            .filter(|t| !self.is_hosted(t))
            .cloned()
            .collect();
        for t in stale {
            info!(ticker = %t, "no book trades this ticker any more, dropping its candle windows");
            self.state_builders.remove(&t);
        }
    }

    fn total_open_positions(&self) -> usize {
        self.books.iter().map(|b| b.open_position_count()).sum()
    }

    async fn persist_all(&self) {
        for book in &self.books {
            for t in book.tickers() {
                let shared = self.shared_for(t);
                book.persist_state(t, &shared).await;
            }
        }
    }

    /// flatten every book (shutdown / signal / end of replay).
    async fn flatten_all_books(&mut self, reason: ExitReason) {
        let now = self.now();
        let Runtime { books, last_prices, .. } = self;
        for book in books.iter_mut() {
            book.flatten_all(reason.clone(), last_prices, now).await;
        }
    }

    fn shutdown_summary(&self) {
        let per_book: Vec<String> = self
            .books
            .iter()
            .map(|b| format!("{}: pnl {:.2}, open {:?}", b.name(), b.daily_pnl(), b.open_tickers()))
            .collect();
        info!(
            total_ticks = self.tick_count,
            daily_pnl = format!("{:.2}", self.primary().daily_pnl()),
            open_positions = self.total_open_positions(),
            books = ?per_book,
            "shutdown complete"
        );
    }
}

/// value following `flag` in argv, if any.
fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

#[tokio::main]
async fn main() {
    // 0. load .env file (ok if missing)
    let _ = dotenvy::dotenv();

    let args: Vec<String> = std::env::args().collect();
    let demo_flag = args.iter().any(|a| a == "--demo");
    let replay_dir = arg_value(&args, "--replay-bars");
    let replay_date = arg_value(&args, "--replay-date");
    let replay_out = arg_value(&args, "--replay-out");
    let replay_slippage_bps: f64 = arg_value(&args, "--replay-slippage-bps")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let mode = match (&replay_dir, demo_flag) {
        (Some(_), _) => Mode::Replay,
        (None, true) => Mode::Demo,
        (None, false) => Mode::Live,
    };
    let demo_mode = mode == Mode::Demo;
    let replay_mode = mode == Mode::Replay;
    let replay_date: Option<NaiveDate> = if replay_mode {
        match replay_date.as_deref().map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d")) {
            Some(Ok(d)) => Some(d),
            _ => {
                eprintln!("--replay-bars needs --replay-date YYYY-MM-DD");
                std::process::exit(2);
            }
        }
    } else {
        None
    };
    if replay_mode && replay_out.is_none() {
        eprintln!("--replay-bars needs --replay-out FILE (trade csv; nothing is written to the database)");
        std::process::exit(2);
    }

    // 1. init structured logging (console + file layers)
    let log_dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    std::fs::create_dir_all(&log_dir).expect("failed to create log directory");

    let log_name = match replay_date {
        Some(d) => format!("{}/replay_{}.log", log_dir, d),
        None => format!("{}/paper_trader_{}.log", log_dir, chrono::Utc::now().format("%Y-%m-%d")),
    };
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_name)
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

    info!(?mode, "galactic trading firm — paper trading engine starting");

    // 2. load environment
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let api_key = std::env::var("APCA_API_KEY_ID").unwrap_or_default();
    let api_secret = std::env::var("APCA_API_SECRET_KEY").unwrap_or_default();
    // replay never places orders anywhere: the broker mode is simulated by construction
    let broker_mode = if replay_mode {
        "simulated".to_string()
    } else {
        std::env::var("BROKER_MODE").unwrap_or_else(|_| "simulated".to_string())
    };

    // credentials are mandatory unless the operator explicitly asked for the synthetic feed
    // (or a replay, which needs no market data connection)
    if mode == Mode::Live && (api_key.is_empty() || api_secret.is_empty()) {
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
    let primary_source = if demo_mode { "demo" } else { "paper" };
    // in demo every book writes source='demo' (synthetic prices are never shadow evidence)
    let shadow_source = if demo_mode { "demo" } else { "shadow" };

    // 3. connect to postgres (replay reads books + configs only)
    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("failed to connect to database");
    info!("connected to database");

    // 4. book roster (exactly one enabled primary) + each book's config.
    //    the primary follows the latest promoted row; a shadow runs its own row or follows too.
    let specs: Vec<BookSpec> = match load_books(&pool).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to load the book roster");
            std::process::exit(1);
        }
    };
    let (promoted_id, promoted_config) = match load_config(&pool).await {
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
    if promoted_id != promoted_config.config_id {
        warn!(
            row_id = promoted_id,
            blob_config_id = promoted_config.config_id,
            "config blob's config_id differs from its row id — using the row id for attribution"
        );
    }

    let mut plans: Vec<BookPlan> = Vec::new();
    for spec in specs {
        let resolved = match spec.config_version_id {
            None => Ok((promoted_id, promoted_config.clone())),
            Some(id) => load_config_by_id(&pool, id).await,
        };
        match resolved {
            Ok((id, config)) => {
                info!(
                    book = %spec.name,
                    role = spec.role.as_str(),
                    config_version_id = id,
                    follows_promoted = spec.follows_promoted(),
                    tickers = ?spec.effective_tickers(&config),
                    capital = ?spec.capital,
                    purpose = %spec.purpose,
                    "book"
                );
                plans.push(BookPlan::new(spec, id, config));
            }
            Err(e) if spec.is_primary() => {
                error!(book = %spec.name, error = %e, "failed to load the primary's config");
                std::process::exit(1);
            }
            Err(e) => {
                error!(book = %spec.name, error = %e, "failed to load the shadow's config — book skipped");
            }
        }
    }
    let limits = Limits::from_env();
    let (plans, skipped) = apply_limits(plans, &limits, CROSS_INDEX_SYMBOL);
    for (plan, why) in skipped {
        error!(book = %plan.spec.name, reason = %why, "shadow book NOT hosted");
    }
    let primary_plan_config = plans[0].config.clone();
    let primary_config_id = plans[0].config_id;

    // 5. resolve capital (alpaca equity only matters for the real paper broker)
    let env_capital: f64 = std::env::var("INITIAL_CAPITAL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100_000.0);
    let capital = if replay_mode {
        env_capital
    } else {
        resolve_capital(&broker_mode, &api_key, &api_secret, env_capital).await
    };
    info!(
        capital,
        env_capital,
        "capital resolved (each ticker engine sizes against this; max_concurrent_positions bounds exposure)"
    );

    // 6. brokers + books. the primary's broker follows BROKER_MODE (the only place an
    //    Alpaca broker is constructed); every shadow builds its own simulated broker.
    let replay_log: ReplayLog = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let process_started_at = Utc::now();
    let mut books: Vec<Book> = Vec::new();
    for plan in plans {
        let name = plan.spec.name.clone();
        let sink = if replay_mode {
            Sink::Replay(replay_log.clone())
        } else {
            let source = if plan.spec.is_primary() { primary_source } else { shadow_source };
            Sink::Live { pool: pool.clone(), trade_writer: TradeWriter::new(pool.clone(), source, &name) }
        };
        if plan.spec.is_primary() {
            let broker: Box<dyn Broker> = match broker_mode.as_str() {
                "alpaca_paper" => {
                    let b = AlpacaBroker::new(api_key.clone(), api_secret.clone())
                        .expect("failed to create alpaca broker");
                    info!("using alpaca paper broker");
                    Box::new(b)
                }
                "simulated" => {
                    let bps = if replay_mode { replay_slippage_bps } else { 5.0 };
                    info!(slippage_bps = bps, "using simulated broker");
                    Box::new(SimulatedBroker::new(bps))
                }
                other => {
                    error!(broker_mode = other, "unknown BROKER_MODE (expected 'simulated' or 'alpaca_paper')");
                    std::process::exit(1);
                }
            };
            match Book::new_primary(plan, broker, capital, sink, process_started_at) {
                Ok(b) => books.push(b),
                Err(e) => {
                    error!(error = %e, "no engines could be built, exiting");
                    std::process::exit(1);
                }
            }
        } else {
            let bps = if replay_mode { replay_slippage_bps } else { SHADOW_SLIPPAGE_BPS };
            match Book::new_shadow(plan, capital, sink, bps, process_started_at) {
                Ok(b) => books.push(b),
                Err(e) => error!(book = %name, error = %e, "shadow book NOT hosted"),
            }
        }
    }
    let roster: Vec<String> = books
        .iter()
        .map(|b| format!("{} [{} cfg {} {:?} {}]", b.name(), b.role().as_str(), b.config_id(), b.tickers(), b.broker_kind()))
        .collect();
    info!(books = ?roster, "book roster");

    // 6b. reconcile: the engine starts flat; anything the primary's broker already holds is
    //     an orphan (shadows are simulated and start empty by construction)
    match books[0].broker_open_positions().await {
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

    // 7. config watcher (keyed on the promoted row id; books that follow promotions consume it)
    let mut config_watcher = ConfigWatcher::new(pool.clone(), promoted_id);

    // 8. data feed and channel: subscriptions = union of every book's tickers + the index
    let (bar_tx, mut bar_rx) = mpsc::channel::<BarEvent>(1000);
    let mut rt = Runtime {
        cross: CrossTracker::new(CROSS_INDEX_SYMBOL),
        state_builders: HashMap::new(),
        last_bar_at: HashMap::new(),
        last_prices: HashMap::new(),
        feed_stale: HashMap::new(),
        last_any_bar_ts: None,
        tick_count: 0,
        mode,
        books,
    };
    let tickers: Vec<String> = rt.hosted_tickers();
    for t in &tickers {
        rt.state_builders.insert(t.clone(), MarketStateBuilder::new(200));
    }
    let mut stream_symbols = tickers.clone();
    stream_symbols.push(CROSS_INDEX_SYMBOL.to_string());
    info!(symbols = ?stream_symbols, "subscription set (union of books' tickers + index)");

    // 8b. state rows of (book, ticker) pairs this process does not host go away now
    if !replay_mode {
        let hosted: Vec<(String, String)> = rt
            .books
            .iter()
            .flat_map(|b| b.tickers().iter().map(move |t| (b.name().to_string(), t.clone())))
            .collect();
        match delete_unhosted_engine_state(&pool, &hosted).await {
            Ok(gone) if !gone.is_empty() => info!(rows = ?gone, "deleted engine_state rows for pairs not hosted"),
            Ok(_) => {}
            Err(e) => warn!(error = %e, "could not clean up engine_state rows"),
        }
    }

    // 9. warm the candle windows with history, then stream
    match mode {
        Mode::Live => {
            let feed = AlpacaFeed::new(api_key.clone(), api_secret.clone(), tickers.clone());
            // the index feeds MarketState.cross only; seed it so a mid-session restart knows
            // today's session open (the tracker filters by eastern date itself).
            match feed.fetch_historical_bars(CROSS_INDEX_SYMBOL, 1).await {
                Ok(candles) => {
                    rt.cross.seed(CROSS_INDEX_SYMBOL, &candles);
                    info!(symbol = CROSS_INDEX_SYMBOL, bars = candles.len(), "cross-context index seeded");
                }
                Err(e) => warn!(symbol = CROSS_INDEX_SYMBOL, error = %e, "cross-context index seed failed — SPY-conditioned windows stay closed until live SPY bars arrive today"),
            }
            for ticker in &tickers {
                info!(ticker = %ticker, days = WARMUP_LOOKBACK_DAYS, "fetching historical bars for warmup");
                match feed.fetch_historical_bars(ticker, WARMUP_LOOKBACK_DAYS).await {
                    Ok(candles) => {
                        let count = candles.len();
                        rt.cross.seed(ticker, &candles);
                        if let Some(builder) = rt.state_builders.get_mut(ticker) {
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

            let feed = AlpacaFeed::new(api_key, api_secret, stream_symbols);
            tokio::spawn(async move {
                if let Err(e) = feed.stream_bars(bar_tx).await {
                    error!(error = %e, "data feed stream failed");
                }
            });
        }
        Mode::Demo => {
            warn!("--demo: starting SYNTHETIC random-walk feed; trades are written with source='demo'");
            let demo_symbols = stream_symbols.clone();
            tokio::spawn(async move {
                let base_prices: HashMap<&str, f64> = [
                    ("SPY", 450.0), ("QQQ", 380.0), ("AAPL", 175.0),
                    ("NVDA", 800.0), ("MSFT", 420.0), ("AMZN", 200.0),
                ]
                .into_iter()
                .collect();

                let mut prices: HashMap<String, f64> = demo_symbols
                    .iter()
                    .map(|t| (t.clone(), base_prices.get(t.as_str()).copied().unwrap_or(100.0)))
                    .collect();

                let mut rng_state: u64 = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64;

                loop {
                    for ticker in &demo_symbols {
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
        Mode::Replay => {
            let dir = replay_dir.clone().expect("replay dir checked above");
            let date = replay_date.expect("replay date checked above");
            let bars = match load_replay_bars(&dir, &stream_symbols, CROSS_INDEX_SYMBOL, date, REPLAY_WARMUP_DAYS) {
                Ok(b) => b,
                Err(e) => {
                    error!(error = %e, "replay: could not load bars");
                    std::process::exit(1);
                }
            };
            for (sym, candles) in &bars.warmup {
                rt.cross.seed(sym, candles);
                if let Some(builder) = rt.state_builders.get_mut(sym) {
                    builder.seed(candles.clone());
                    info!(ticker = %sym, one_minute_bars = candles.len(), windows = ?builder.window_sizes(), "replay warmup complete");
                } else {
                    info!(symbol = %sym, one_minute_bars = candles.len(), "replay: cross-context symbol seeded");
                }
            }
            info!(date = %date, bars = bars.day.len(), symbols = ?stream_symbols, "replay: feeding the day's bars through the live loop");
            let day = bars.day;
            tokio::spawn(async move {
                for ev in day {
                    if bar_tx.send(ev).await.is_err() {
                        return;
                    }
                }
                // dropping the sender ends the stream: the loop flattens at the last bar
            });
        }
    }

    // 10. timers (both arms are disabled in replay: bar time drives everything)
    let mut config_reload_interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    config_reload_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    heartbeat_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // initial heartbeat rows so observers see the process immediately
    rt.persist_all().await;

    // 10b. launch TUI if feature is enabled (wired to the primary book)
    #[cfg(feature = "tui")]
    let tui_shutdown = Arc::new(AtomicBool::new(false));
    #[cfg(feature = "tui")]
    let dashboard_state = Arc::new(RwLock::new(DashboardState::new(primary_config_id)));
    #[cfg(feature = "tui")]
    {
        dashboard_state
            .write()
            .unwrap()
            .set_strategy_config(primary_plan_config.clone());

        let state = Arc::clone(&dashboard_state);
        let shutdown = Arc::clone(&tui_shutdown);
        std::thread::spawn(move || {
            if let Err(e) = data_feed::tui::run_tui(state, shutdown) {
                eprintln!("TUI error: {e}");
            }
        });
    }
    #[cfg(not(feature = "tui"))]
    let _ = (primary_plan_config, primary_config_id);

    // register SIGTERM handler for graceful container shutdown
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("failed to register SIGTERM handler");

    info!("entering main trading loop");
    let mut feed_closed = false;

    // 11. main select! loop
    loop {
        #[cfg(feature = "tui")]
        if tui_shutdown.load(Ordering::Relaxed) {
            info!("TUI requested shutdown");
            rt.flatten_all_books(ExitReason::ManualOverride).await;
            rt.shutdown_summary();
            break;
        }

        tokio::select! {
            // ── incoming bar from data feed ──
            received = bar_rx.recv(), if !feed_closed => {
                let Some(bar_event) = received else {
                    feed_closed = true;
                    if replay_mode {
                        info!("replay: bars exhausted — flattening at the last bar");
                        rt.flatten_all_books(ExitReason::SessionClose).await;
                        rt.shutdown_summary();
                        break;
                    }
                    warn!("bar channel closed (feed task ended); heartbeats continue");
                    continue;
                };
                let ticker = bar_event.symbol.clone();
                let bar_ts = bar_event.candle.timestamp;

                // only regular-hours bars reach the engine (demo bars are exempt)
                if !demo_mode && !is_regular_hours(bar_ts) {
                    debug!(ticker = %ticker, ts = %bar_ts, "ignoring extended-hours bar");
                    continue;
                }

                // every regular-hours bar (index included) feeds the cross-context tracker
                rt.cross.on_bar(&ticker, &bar_event.candle);
                if rt.last_any_bar_ts.is_none_or(|t| bar_ts > t) {
                    rt.last_any_bar_ts = Some(bar_ts);
                }
                if !rt.is_hosted(&ticker) {
                    debug!(symbol = %ticker, "index/peer bar recorded (not traded)");
                    continue;
                }

                rt.tick_count += 1;
                rt.last_bar_at.insert(ticker.clone(), bar_ts);
                rt.last_prices.insert(ticker.clone(), bar_event.candle.close);
                if rt.feed_stale.remove(&ticker).unwrap_or(false) {
                    info!(ticker = %ticker, "feed resumed");
                }
                for book in &rt.books {
                    book.set_last_price_for(&ticker, bar_event.candle.close);
                }

                #[cfg(feature = "tui")]
                let candle_for_tui = bar_event.candle.clone();

                // build market state from the candle — once per ticker, shared by every book
                let market = match rt.state_builders.get_mut(&ticker) {
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
                let shared = rt.shared_for(&ticker);

                // fan out to every book that trades the ticker; peers = that book's tickers
                let targets: Vec<usize> = rt.books.iter().enumerate().filter(|(_, b)| b.trades(&ticker)).map(|(i, _)| i).collect();
                let mut market = Some(market);
                let mut any_dropped = false;
                #[cfg(feature = "tui")]
                let mut primary_outcome = None;
                for (k, i) in targets.iter().enumerate() {
                    let mut m = if k + 1 == targets.len() {
                        market.take().expect("market consumed once")
                    } else {
                        market.as_ref().expect("market present").clone()
                    };
                    m.cross = rt.cross.context_for(&ticker, rt.books[*i].tickers(), bar_ts);
                    if let Some(outcome) = rt.books[*i].on_bar(&ticker, m, &shared).await {
                        any_dropped |= !outcome.dropped.is_empty();
                        #[cfg(feature = "tui")]
                        if rt.books[*i].is_primary() {
                            primary_outcome = Some(outcome);
                        }
                    }
                }
                if any_dropped {
                    rt.prune_unhosted_builders();
                }

                // update TUI dashboard state (primary book only)
                #[cfg(feature = "tui")]
                if let Some(outcome) = primary_outcome {
                    let primary = rt.primary();
                    let mut dash = dashboard_state.write().unwrap();
                    dash.total_ticks = rt.tick_count;
                    dash.daily_pnl = primary.daily_pnl();

                    let pos_display = primary
                        .session(&ticker)
                        .and_then(|s| s.current_position())
                        .map(|pos| PositionDisplay {
                            direction: pos.direction,
                            entry_price: pos.entry_price,
                            unrealized_pnl: pos.unrealized_pnl,
                            unrealized_pnl_pct: pos.unrealized_pnl_pct,
                            hold_duration_ms: pos.hold_duration_ms,
                        });

                    dash.ensure_ticker_order(&ticker);
                    let last_price = candle_for_tui.close;
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
                    ts.last_price = last_price;
                    ts.composite_score = outcome.result.scores.composite;
                    ts.one_minute_score = outcome.result.scores.one_minute;
                    ts.five_minute_score = outcome.result.scores.five_minute;
                    ts.one_hour_score = outcome.result.scores.one_hour;
                    ts.position = pos_display;
                    ts.price_history.push(last_price);
                    if ts.price_history.len() > 60 {
                        ts.price_history.remove(0);
                    }
                    ts.candle_history.push(candle_for_tui.clone());
                    if ts.candle_history.len() > 60 {
                        ts.candle_history.remove(0);
                    }
                    if let Some(tws) = &outcome.trade {
                        dash.add_trade(TradeLogEntry {
                            ticker: tws.trade.ticker.clone(),
                            direction: format!("{:?}", tws.trade.direction),
                            pnl: tws.trade.pnl,
                            pnl_pct: tws.trade.pnl_pct,
                            exit_reason: format!("{:?}", tws.trade.exit_reason),
                            time: tws.trade.exit_time,
                        });
                    }
                }

                // periodic status log
                if rt.tick_count.is_multiple_of(60) {
                    let positions: Vec<String> = rt
                        .books
                        .iter()
                        .map(|b| format!("{}: {:?}", b.name(), b.open_tickers()))
                        .collect();
                    let pending: Vec<String> = rt
                        .books
                        .iter()
                        .filter_map(|b| b.pending_id().map(|id| format!("{}: {}", b.name(), id)))
                        .collect();
                    info!(
                        ticks = rt.tick_count,
                        daily_pnl = format!("{:.2}", rt.primary().daily_pnl()),
                        open_positions = ?positions,
                        pending_config = ?pending,
                        "status"
                    );
                }
            }

            // ── heartbeat: staleness, missed-close safety net, state rows ──
            _ = heartbeat_interval.tick(), if !replay_mode => {
                let now = Utc::now();
                let market_open_now = is_regular_hours(now);

                // feed staleness per hosted ticker
                if market_open_now && !demo_mode {
                    let tickers: Vec<String> = rt.hosted_tickers();
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
                // engine never sees the force_exit_by tick. close it from the clock instead —
                // per book, each with its own force_exit_by.
                let mut any_dropped = false;
                {
                    let Runtime { books, last_prices, .. } = &mut rt;
                    for book in books.iter_mut() {
                        if let Some(cutoff) = parse_hm(book.force_exit_by()) {
                            if eastern_minutes(now) >= cutoff + 2 && book.open_position_count() > 0 {
                                warn!(book = %book.name(), "positions still open past force_exit_by — flattening from the clock");
                                book.flatten_all(ExitReason::SessionClose, last_prices, now).await;
                                if book.has_pending() {
                                    any_dropped |= !book.apply_pending_where_flat().await.is_empty();
                                }
                            }
                        }
                    }
                }
                if any_dropped {
                    rt.prune_unhosted_builders();
                }

                rt.persist_all().await;
            }

            // ── config reload timer: books that follow promotions take the new row ──
            _ = config_reload_interval.tick(), if !replay_mode => {
                match config_watcher.check_for_update().await {
                    Ok(Some((version_id, new_config))) => {
                        let mut any_dropped = false;
                        for book in rt.books.iter_mut().filter(|b| b.follows_promoted()) {
                            book.set_pending(version_id, new_config.clone()).await;
                            any_dropped |= !book.apply_pending_where_flat().await.is_empty();
                        }
                        config_watcher.acknowledge(version_id);
                        if any_dropped {
                            rt.prune_unhosted_builders();
                        }

                        #[cfg(feature = "tui")]
                        {
                            let mut dash = dashboard_state.write().unwrap();
                            dash.config_version = version_id;
                            dash.set_strategy_config(new_config);
                        }
                        rt.persist_all().await;
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
                rt.flatten_all_books(ExitReason::ManualOverride).await;
                rt.shutdown_summary();
                break;
            }

            // ── graceful shutdown on SIGTERM (docker stop / kill -TERM) ──
            _ = sigterm.recv() => {
                info!("SIGTERM received, shutting down");
                #[cfg(feature = "tui")]
                tui_shutdown.store(true, Ordering::Relaxed);
                rt.flatten_all_books(ExitReason::ManualOverride).await;
                rt.shutdown_summary();
                break;
            }
        }
    }

    if replay_mode {
        // the only output of a replay: one trade csv, never the database
        let trades = replay_log.lock().map(|v| v.clone()).unwrap_or_default();
        let out = replay_out.expect("replay out checked above");
        match write_replay_trades(&out, &trades) {
            Ok(()) => info!(path = %out, trades = trades.len(), "replay: trades written"),
            Err(e) => {
                error!(path = %out, error = %e, "replay: failed to write trades");
                std::process::exit(1);
            }
        }
        return;
    }

    // final state rows so observers see the process is gone cleanly
    rt.persist_all().await;
}
