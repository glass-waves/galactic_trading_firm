//! books: the engines the live trader hosts.
//!
//! the **primary** book is the latest promoted config on the broker `BROKER_MODE` selects
//! (today's behaviour, unchanged). **shadow** books run a `config_versions` row or a ticker
//! set on the same bars at the same moment, each on its own `SimulatedBroker` — by
//! construction: `Book::new_shadow` builds the broker itself and no shadow constructor
//! accepts one, so a shadow can never reach Alpaca whatever the environment says.
//!
//! everything that is per-strategy lives here (sessions, capacity, daily P&L, pending
//! config, gate/near-miss dedupe, trade + state writers); everything that is
//! per-symbol (candle windows, cross tracker, last bar / price / staleness) stays in
//! the runtime and is shared by every book.
//!
//! see docs/plans/2026-09-26_pipeline_and_books.md §3 and docs/dev/books_trader_notes.md.

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, NaiveDate, Utc};
use tracing::{debug, error, info, warn};
use types::action::ExitReason;
use types::config::StrategyConfig;
use types::market::MarketState;
use types::tick_result::{TickEvent, TickResult};

use crate::broker::{Broker, BrokerError, BrokerPosition, SimulatedBroker};
use crate::config_watcher::try_build_engine;
use crate::live_session::{LiveSession, TradeWithScores};
use crate::session_clock::eastern_date;
use crate::state_writer::{
    delete_engine_state, upsert_engine_state, write_book_session, write_entry_block_event,
    BlockEvent, BlockKind, EngineStateRow, PositionSnapshot,
};
use crate::trade_writer::TradeWriter;

/// the reserved name of the primary book (`books.name`).
pub const PRIMARY_BOOK: &str = "primary";
/// `MAX_BOOKS` default: primary + shadows hosted by one process.
pub const DEFAULT_MAX_BOOKS: usize = 8;
/// `MAX_SYMBOLS` default: websocket symbols (union of every book's tickers + the index).
/// the free IEX plan allows 30 per connection.
pub const DEFAULT_MAX_SYMBOLS: usize = 25;
/// slippage of every shadow book's simulated broker, same as the primary's simulated mode.
pub const SHADOW_SLIPPAGE_BPS: f64 = 5.0;
/// a near-miss diagnostic is persisted at most this often per ticker.
const NEAR_MISS_THROTTLE_SECS: i64 = 300;
/// a repeated gate reason is re-persisted at most this often per ticker.
const GATE_REPEAT_SECS: i64 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookRole {
    Primary,
    Shadow,
}

impl BookRole {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "primary" => Some(BookRole::Primary),
            "shadow" => Some(BookRole::Shadow),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            BookRole::Primary => "primary",
            BookRole::Shadow => "shadow",
        }
    }
}

/// one enabled row of `books`.
#[derive(Debug, Clone)]
pub struct BookSpec {
    pub name: String,
    pub role: BookRole,
    /// `None` = follow the latest promoted config (like the primary).
    pub config_version_id: Option<i64>,
    /// `None` = the config's own tickers.
    pub tickers: Option<Vec<String>>,
    /// `None` = the process capital.
    pub capital: Option<f64>,
    pub purpose: String,
}

impl BookSpec {
    pub fn is_primary(&self) -> bool {
        self.role == BookRole::Primary
    }

    /// a book without a pinned config row follows promotions.
    pub fn follows_promoted(&self) -> bool {
        self.config_version_id.is_none()
    }

    /// the ticker list this book trades under `config`: the row's override or the config's own.
    pub fn effective_tickers(&self, config: &StrategyConfig) -> Vec<String> {
        match &self.tickers {
            Some(t) if !t.is_empty() => t.clone(),
            _ => config.tickers.clone(),
        }
    }

    /// apply the ticker override to a freshly loaded config (also on every hot reload).
    pub fn apply_to(&self, config: &mut StrategyConfig) {
        config.tickers = self.effective_tickers(config);
    }
}

#[derive(Debug)]
pub enum BookError {
    NoPrimary,
    MultiplePrimaries(Vec<String>),
    BadRole { name: String, role: String },
    Database(sqlx::Error),
    /// no ticker engine could be built for the book.
    NoEngines(String),
}

impl fmt::Display for BookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BookError::NoPrimary => write!(f, "books: no enabled primary row"),
            BookError::MultiplePrimaries(names) => {
                write!(f, "books: more than one enabled primary: {names:?}")
            }
            BookError::BadRole { name, role } => write!(f, "books: row {name} has role {role:?}"),
            BookError::Database(e) => write!(f, "books: database error: {e}"),
            BookError::NoEngines(name) => write!(f, "book {name}: no engine could be built"),
        }
    }
}

impl std::error::Error for BookError {}

impl From<sqlx::Error> for BookError {
    fn from(e: sqlx::Error) -> Self {
        BookError::Database(e)
    }
}

/// enabled rows of `books`, validated by [`validate_roster`].
pub async fn load_books(pool: &sqlx::PgPool) -> Result<Vec<BookSpec>, BookError> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, Option<i64>, Option<Vec<String>>, Option<f64>, String)> =
        sqlx::query_as(
            "SELECT name, role, config_version_id, tickers, capital, purpose \
             FROM books WHERE enabled ORDER BY created_at, name",
        )
        .fetch_all(pool)
        .await?;
    let mut specs = Vec::with_capacity(rows.len());
    for (name, role, config_version_id, tickers, capital, purpose) in rows {
        let role = BookRole::parse(&role).ok_or_else(|| BookError::BadRole {
            name: name.clone(),
            role: role.clone(),
        })?;
        specs.push(BookSpec { name, role, config_version_id, tickers, capital, purpose });
    }
    validate_roster(specs)
}

/// exactly one primary, returned first; shadows keep their order.
pub fn validate_roster(specs: Vec<BookSpec>) -> Result<Vec<BookSpec>, BookError> {
    let primaries: Vec<String> =
        specs.iter().filter(|s| s.is_primary()).map(|s| s.name.clone()).collect();
    match primaries.len() {
        0 => return Err(BookError::NoPrimary),
        1 => {}
        _ => return Err(BookError::MultiplePrimaries(primaries)),
    }
    let (mut primary, shadows): (Vec<BookSpec>, Vec<BookSpec>) =
        specs.into_iter().partition(|s| s.is_primary());
    primary.extend(shadows);
    Ok(primary)
}

/// hosting limits (`MAX_BOOKS`, `MAX_SYMBOLS`).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_books: usize,
    pub max_symbols: usize,
}

impl Limits {
    pub fn from_env() -> Self {
        let read = |key: &str, default: usize| {
            std::env::var(key)
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .filter(|v| *v > 0)
                .unwrap_or(default)
        };
        Self {
            max_books: read("MAX_BOOKS", DEFAULT_MAX_BOOKS),
            max_symbols: read("MAX_SYMBOLS", DEFAULT_MAX_SYMBOLS),
        }
    }
}

/// a book whose config is resolved (ticker override applied) but whose engines are not built.
#[derive(Debug, Clone)]
pub struct BookPlan {
    pub spec: BookSpec,
    pub config_id: i64,
    pub config: StrategyConfig,
}

impl BookPlan {
    pub fn new(spec: BookSpec, config_id: i64, mut config: StrategyConfig) -> Self {
        spec.apply_to(&mut config);
        Self { spec, config_id, config }
    }
}

/// apply the hosting limits. the primary (first) is always kept; a shadow that would
/// exceed `max_books`, or push the symbol union (books' tickers + `index`) past
/// `max_symbols`, is returned in the skipped list with the reason.
pub fn apply_limits(
    plans: Vec<BookPlan>,
    limits: &Limits,
    index: &str,
) -> (Vec<BookPlan>, Vec<(BookPlan, String)>) {
    let mut kept: Vec<BookPlan> = Vec::new();
    let mut skipped: Vec<(BookPlan, String)> = Vec::new();
    let mut symbols: BTreeSet<String> = BTreeSet::new();
    symbols.insert(index.to_string());
    for plan in plans {
        if plan.spec.is_primary() {
            symbols.extend(plan.config.tickers.iter().cloned());
            kept.push(plan);
            continue;
        }
        if kept.len() >= limits.max_books {
            skipped.push((plan, format!("MAX_BOOKS={} reached", limits.max_books)));
            continue;
        }
        let mut union = symbols.clone();
        union.extend(plan.config.tickers.iter().cloned());
        // a shadow that adds no symbol never costs a subscription, whatever the primary needs
        if union.len() > limits.max_symbols && union.len() > symbols.len() {
            skipped.push((
                plan,
                format!(
                    "would need {} websocket symbols > MAX_SYMBOLS={}",
                    union.len(),
                    limits.max_symbols
                ),
            ));
            continue;
        }
        symbols = union;
        kept.push(plan);
    }
    (kept, skipped)
}

/// calendar days of 1-minute history a book gets when its config does not set
/// `session.warmup_days` — what every live day and every 8-day replay has used.
pub const DEFAULT_WARMUP_DAYS: u32 = 8;

/// one book's say in the warm-up: its name, role, tickers and `session.warmup_days`.
#[derive(Debug, Clone, Copy)]
pub struct WarmupInput<'a> {
    pub book: &'a str,
    pub is_primary: bool,
    pub tickers: &'a [String],
    pub warmup_days: Option<u32>,
}

impl<'a> WarmupInput<'a> {
    pub fn from_book(b: &'a Book) -> Self {
        Self {
            book: b.name(),
            is_primary: b.is_primary(),
            tickers: b.tickers(),
            warmup_days: b.config().session.warmup_days,
        }
    }

    fn days(&self) -> u32 {
        self.warmup_days.unwrap_or(DEFAULT_WARMUP_DAYS)
    }
}

/// a symbol the primary trades whose warm-up another book (or the primary's own config)
/// raised above [`DEFAULT_WARMUP_DAYS`]: the primary's windows for it no longer match the
/// 8-day replays its record is built on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarmupRaise {
    pub symbol: String,
    pub days: u32,
    pub book: String,
}

/// per-symbol warm-up, computed once at start-up (a hot-reloaded `warmup_days` applies at
/// the next start).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WarmupPlan {
    /// every hosted ticker + the index → calendar days of history to load.
    pub days: std::collections::BTreeMap<String, u32>,
    /// primary-traded symbols raised above the default (one parity warning each).
    pub raised: Vec<WarmupRaise>,
}

impl WarmupPlan {
    pub fn days_for(&self, symbol: &str) -> u32 {
        self.days.get(symbol).copied().unwrap_or(DEFAULT_WARMUP_DAYS)
    }
}

/// the per-symbol max rule. a traded symbol gets the max over the books trading it of
/// (`session.warmup_days` or 8); the index, when no book trades it, gets the max over all
/// books (it only seeds the cross tracker, which reads today's session and the prior
/// close, so its length changes nothing a book computes); a traded index follows the
/// traded rule — its candle windows are what the books trading it read.
pub fn warmup_plan(books: &[WarmupInput<'_>], index: &str) -> WarmupPlan {
    // symbol → (days, book that set it); the earlier book wins a tie (the primary is first)
    let mut traded: std::collections::BTreeMap<String, (u32, String)> = Default::default();
    for b in books {
        let d = b.days();
        for t in b.tickers {
            match traded.get(t) {
                Some((cur, _)) if *cur >= d => {}
                _ => {
                    traded.insert(t.clone(), (d, b.book.to_string()));
                }
            }
        }
    }
    let mut plan = WarmupPlan::default();
    let primary_tickers: BTreeSet<&String> =
        books.iter().filter(|b| b.is_primary).flat_map(|b| b.tickers.iter()).collect();
    for (sym, (d, by)) in &traded {
        plan.days.insert(sym.clone(), *d);
        if *d > DEFAULT_WARMUP_DAYS && primary_tickers.contains(sym) {
            plan.raised.push(WarmupRaise { symbol: sym.clone(), days: *d, book: by.clone() });
        }
    }
    if !traded.contains_key(index) {
        let d = books.iter().map(|b| b.days()).max().unwrap_or(DEFAULT_WARMUP_DAYS);
        plan.days.insert(index.to_string(), d);
    }
    plan
}

/// a trade the replay harness collected instead of writing to postgres.
#[derive(Debug, Clone)]
pub struct ReplayTrade {
    pub book: String,
    pub tws: TradeWithScores,
    pub broker_exit_price: Option<f64>,
}

pub type ReplayLog = Arc<Mutex<Vec<ReplayTrade>>>;

/// where a book's output goes.
pub enum Sink {
    /// live / demo: trades, state rows, block events and session rows go to postgres.
    Live { pool: sqlx::PgPool, trade_writer: TradeWriter },
    /// replay harness: trades are collected in memory; nothing touches the database.
    Replay(ReplayLog),
}

impl Sink {
    fn pool(&self) -> Option<&sqlx::PgPool> {
        match self {
            Sink::Live { pool, .. } => Some(pool),
            Sink::Replay(_) => None,
        }
    }
}

/// per-symbol facts the runtime owns, handed to a book for its state row.
#[derive(Debug, Clone, Copy, Default)]
pub struct TickerShared {
    pub last_bar_at: Option<DateTime<Utc>>,
    pub last_price: Option<f64>,
    pub feed_stale: bool,
}

/// what one bar did to one book (for the TUI and the runtime's builder pruning).
pub struct BarOutcome {
    pub result: TickResult,
    pub trade: Option<TradeWithScores>,
    /// tickers whose session was dropped by a pending config swap on this bar.
    pub dropped: Vec<String>,
}

pub struct Book {
    spec: BookSpec,
    /// config currently driving new engines (ticker override applied).
    config: StrategyConfig,
    config_id: i64,
    /// config waiting for in-position tickers to go flat.
    pending: Option<(i64, StrategyConfig)>,
    sessions: HashMap<String, LiveSession>,
    /// sorted hosted tickers (peer universe for the cross context).
    ticker_list: Vec<String>,
    broker: Box<dyn Broker>,
    broker_mode: String,
    sink: Sink,
    capital: f64,
    daily_pnl: f64,
    daily_pnl_date: Option<NaiveDate>,
    /// eastern date whose `book_sessions` row has been written.
    session_row_date: Option<NaiveDate>,
    /// last persisted gate reason and when, per ticker (dedupe).
    last_gate: HashMap<String, (String, DateTime<Utc>)>,
    /// last persisted near-miss time, per ticker (throttle).
    last_near_miss: HashMap<String, DateTime<Utc>>,
    process_started_at: DateTime<Utc>,
}

impl Book {
    /// the primary book on the broker the caller chose from `BROKER_MODE` — the only
    /// constructor that accepts a broker, hence the only way an Alpaca broker enters a book.
    pub fn new_primary(
        plan: BookPlan,
        broker: Box<dyn Broker>,
        capital: f64,
        sink: Sink,
        process_started_at: DateTime<Utc>,
    ) -> Result<Self, BookError> {
        let mode = broker.kind().to_string();
        Self::build(plan, broker, mode, capital, sink, process_started_at)
    }

    /// a shadow book: always its own `SimulatedBroker`, whatever `BROKER_MODE` says.
    pub fn new_shadow(
        plan: BookPlan,
        capital: f64,
        sink: Sink,
        slippage_bps: f64,
        process_started_at: DateTime<Utc>,
    ) -> Result<Self, BookError> {
        let broker: Box<dyn Broker> = Box::new(SimulatedBroker::new(slippage_bps));
        Self::build(plan, broker, "simulated".to_string(), capital, sink, process_started_at)
    }

    fn build(
        plan: BookPlan,
        broker: Box<dyn Broker>,
        broker_mode: String,
        capital: f64,
        sink: Sink,
        process_started_at: DateTime<Utc>,
    ) -> Result<Self, BookError> {
        let BookPlan { spec, config_id, config } = plan;
        let capital = spec.capital.unwrap_or(capital);
        let mut sessions = HashMap::new();
        for ticker in &config.tickers {
            match try_build_engine(&config, ticker, capital) {
                Some(engine) => {
                    info!(book = %spec.name, ticker = %ticker, config_version_id = config_id, "engine built");
                    sessions.insert(ticker.clone(), LiveSession::new(engine, config_id));
                }
                None => {
                    error!(book = %spec.name, ticker = %ticker, "failed to build engine, skipping ticker");
                }
            }
        }
        if sessions.is_empty() {
            return Err(BookError::NoEngines(spec.name.clone()));
        }
        let mut book = Self {
            spec,
            config,
            config_id,
            pending: None,
            sessions,
            ticker_list: Vec::new(),
            broker,
            broker_mode,
            sink,
            capital,
            daily_pnl: 0.0,
            daily_pnl_date: None,
            session_row_date: None,
            last_gate: HashMap::new(),
            last_near_miss: HashMap::new(),
            process_started_at,
        };
        book.refresh_ticker_list();
        Ok(book)
    }

    fn refresh_ticker_list(&mut self) {
        let mut t: Vec<String> = self.sessions.keys().cloned().collect();
        t.sort();
        self.ticker_list = t;
    }

    // ── accessors ──

    pub fn name(&self) -> &str {
        &self.spec.name
    }

    pub fn spec(&self) -> &BookSpec {
        &self.spec
    }

    pub fn role(&self) -> BookRole {
        self.spec.role
    }

    pub fn is_primary(&self) -> bool {
        self.spec.is_primary()
    }

    pub fn follows_promoted(&self) -> bool {
        self.spec.follows_promoted()
    }

    pub fn config(&self) -> &StrategyConfig {
        &self.config
    }

    pub fn config_id(&self) -> i64 {
        self.config_id
    }

    pub fn pending_id(&self) -> Option<i64> {
        self.pending.as_ref().map(|(id, _)| *id)
    }

    pub fn pending_config(&self) -> Option<&StrategyConfig> {
        self.pending.as_ref().map(|(_, c)| c)
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// hosted tickers (sorted) — the peer universe for this book's cross context.
    pub fn tickers(&self) -> &[String] {
        &self.ticker_list
    }

    pub fn trades(&self, ticker: &str) -> bool {
        self.sessions.contains_key(ticker)
    }

    pub fn session(&self, ticker: &str) -> Option<&LiveSession> {
        self.sessions.get(ticker)
    }

    pub fn open_position_count(&self) -> usize {
        self.sessions.values().filter(|s| s.has_position()).count()
    }

    pub fn open_tickers(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .sessions
            .iter()
            .filter(|(_, s)| s.has_position())
            .map(|(t, _)| t.clone())
            .collect();
        v.sort();
        v
    }

    pub fn daily_pnl(&self) -> f64 {
        self.daily_pnl
    }

    pub fn capital(&self) -> f64 {
        self.capital
    }

    pub fn broker_kind(&self) -> &'static str {
        self.broker.kind()
    }

    pub fn broker_mode(&self) -> &str {
        &self.broker_mode
    }

    /// the current config's `force_exit_by` (books may differ; the clock safety net uses it).
    pub fn force_exit_by(&self) -> &str {
        &self.config.session.force_exit_by
    }

    pub fn set_last_price_for(&self, ticker: &str, price: f64) {
        self.broker.set_last_price_for(ticker, price);
    }

    /// positions the book's broker holds (startup reconciliation, primary only).
    pub async fn broker_open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError> {
        self.broker.open_positions().await
    }

    fn max_concurrent(&self) -> usize {
        self.config.session.max_concurrent_positions as usize
    }

    // ── per-bar ──

    /// roll the book's daily P&L on the first bar of a new eastern day.
    fn roll_daily_pnl(&mut self, ts: DateTime<Utc>) {
        let d = eastern_date(ts);
        if self.daily_pnl_date != Some(d) {
            if self.daily_pnl_date.is_some() {
                info!(book = %self.spec.name, date = %d, prev_daily_pnl = format!("{:.2}", self.daily_pnl), "new trading day");
            }
            self.daily_pnl_date = Some(d);
            self.daily_pnl = 0.0;
        }
    }

    /// `book_sessions` row on the book's first bar of the eastern date (live sink only).
    async fn write_session_row(&mut self, ts: DateTime<Utc>) {
        let d = eastern_date(ts);
        if self.session_row_date == Some(d) {
            return;
        }
        let Some(pool) = self.sink.pool() else {
            self.session_row_date = Some(d);
            return;
        };
        match write_book_session(pool, &self.spec.name, d, ts, self.config_id).await {
            Ok(inserted) => {
                if inserted {
                    info!(book = %self.spec.name, session_date = %d, config_version_id = self.config_id, "book session recorded");
                }
                self.session_row_date = Some(d);
            }
            Err(e) => warn!(book = %self.spec.name, error = %e, "failed to write book_sessions row"),
        }
    }

    /// what the primary did per ticker before books existed: capacity check, tick, broker
    /// submit/close on open/close (undo on broker failure), daily P&L, trade write, pending
    /// swap when flat, block events, state row. `None` when the book does not trade `ticker`.
    pub async fn on_bar(
        &mut self,
        ticker: &str,
        mut market: MarketState,
        shared: &TickerShared,
    ) -> Option<BarOutcome> {
        if !self.sessions.contains_key(ticker) {
            return None;
        }
        let bar_ts = market.timestamp;
        let last_price = market.last_price;
        self.roll_daily_pnl(bar_ts);
        self.write_session_row(bar_ts).await;

        // cross-ticker capacity: still tick (exits, clocks, diagnostics), but block entries
        let at_capacity = self.open_position_count() >= self.max_concurrent();
        let (result, trade, session_cfg_id) = {
            let session = self.sessions.get_mut(ticker)?;
            if at_capacity && !session.has_position() {
                market.entries_blocked = true;
            }
            let (result, trade) = session.on_tick(&mut market);
            (result, trade, session.config_version_id())
        };

        if let TickEvent::PositionOpened = &result.event {
            let opened = self
                .sessions
                .get(ticker)
                .and_then(|s| s.current_position())
                .map(|p| (p.size, p.direction));
            if let Some((size, direction)) = opened {
                // engine sizes in whole shares already; the broker gets exactly that
                let shares = size.floor();
                info!(
                    book = %self.spec.name,
                    ticker = %ticker,
                    composite = result.scores.composite,
                    price = market.last_price,
                    shares,
                    entry_reason = %result.entry_reason,
                    "position opened"
                );
                if shares < 1.0 {
                    error!(book = %self.spec.name, ticker = %ticker, size, "engine opened a sub-share position, undoing");
                    if let Some(s) = self.sessions.get_mut(ticker) {
                        s.undo_open();
                    }
                } else {
                    match self.broker.submit_order(ticker, direction, shares).await {
                        Ok(fill) => {
                            info!(
                                book = %self.spec.name,
                                ticker = %ticker,
                                fill_price = fill.fill_price,
                                quantity = fill.quantity,
                                engine_price = market.last_price,
                                "broker order filled"
                            );
                            if let Some(s) = self.sessions.get_mut(ticker) {
                                s.set_broker_entry_price(fill.fill_price);
                            }
                        }
                        Err(e) => {
                            error!(book = %self.spec.name, ticker = %ticker, error = %e, "broker order failed, undoing position");
                            if let Some(s) = self.sessions.get_mut(ticker) {
                                s.undo_open();
                            }
                        }
                    }
                }
            }
        }

        let mut dropped = Vec::new();
        if let Some(tws) = &trade {
            let broker_exit = match self.broker.close_position(&tws.trade.ticker).await {
                Ok(fill) => {
                    info!(book = %self.spec.name, ticker = %tws.trade.ticker, fill_price = fill.fill_price, engine_price = tws.trade.exit_price, "broker position closed");
                    Some(fill.fill_price)
                }
                Err(e) => {
                    error!(book = %self.spec.name, ticker = %tws.trade.ticker, error = %e, "broker close FAILED (engine already closed) — check the account");
                    None
                }
            };
            self.daily_pnl += tws.trade.pnl;
            self.record_trade(tws, broker_exit).await;
            // this ticker is flat now — if a config is waiting, swap it in
            if self.pending.is_some() {
                dropped = self.apply_pending_where_flat().await;
            }
        }

        // observability: gate/near-miss events + state row
        self.record_block_events(ticker, bar_ts, &result, last_price, session_cfg_id).await;
        self.persist_state(ticker, shared).await;

        Some(BarOutcome { result, trade, dropped })
    }

    // ── trades ──

    async fn record_trade(&self, tws: &TradeWithScores, broker_exit_price: Option<f64>) {
        info!(
            book = %self.spec.name,
            ticker = %tws.trade.ticker,
            pnl = format!("{:.2}", tws.trade.pnl),
            pnl_pct = format!("{:.2}%", tws.trade.pnl_pct * 100.0),
            exit_reason = ?tws.trade.exit_reason,
            entry_reason = %tws.entry_reason,
            hold_ms = tws.trade.hold_duration_ms,
            config_version = tws.config_version_id,
            "trade completed"
        );
        match &self.sink {
            Sink::Live { trade_writer, .. } => match trade_writer.write_trade(tws, broker_exit_price).await {
                Ok(trade_id) => info!(book = %self.spec.name, trade_id, "trade written to database"),
                Err(e) => error!(book = %self.spec.name, error = %e, "failed to write trade to database"),
            },
            Sink::Replay(log) => {
                if let Ok(mut v) = log.lock() {
                    v.push(ReplayTrade { book: self.spec.name.clone(), tws: tws.clone(), broker_exit_price });
                }
            }
        }
    }

    /// close the position at the broker and record the trade. `price` is the engine-side
    /// exit price (last known bar close) used when the broker has no better number;
    /// `now` stamps the exit (wall clock live, bar time in replay).
    async fn close_and_record(
        &mut self,
        ticker: &str,
        price: f64,
        reason: ExitReason,
        now: DateTime<Utc>,
    ) {
        let broker_exit = match self.broker.close_position(ticker).await {
            Ok(fill) => {
                info!(book = %self.spec.name, ticker, fill_price = fill.fill_price, ?reason, "broker position closed");
                Some(fill.fill_price)
            }
            Err(e) => {
                error!(book = %self.spec.name, ticker, error = %e, ?reason, "broker close FAILED — check the account manually");
                None
            }
        };
        let engine_price = if price > 0.0 { price } else { broker_exit.unwrap_or(0.0) };
        let closed = self
            .sessions
            .get_mut(ticker)
            .and_then(|s| s.force_close(engine_price, now, reason));
        match closed {
            Some(tws) => {
                self.daily_pnl += tws.trade.pnl;
                self.record_trade(&tws, broker_exit).await;
            }
            None => warn!(book = %self.spec.name, ticker, "force_close produced no trade (engine had no position)"),
        }
    }

    /// flatten every open position (shutdown / emergency / missed close).
    pub async fn flatten_all(
        &mut self,
        reason: ExitReason,
        last_prices: &HashMap<String, f64>,
        now: DateTime<Utc>,
    ) {
        for ticker in self.open_tickers() {
            let price = last_prices.get(&ticker).copied().unwrap_or(0.0);
            warn!(book = %self.spec.name, ticker = %ticker, ?reason, "force-closing open position");
            self.close_and_record(&ticker, price, reason.clone(), now).await;
        }
    }

    // ── hot reload ──

    /// a newer promoted config: re-apply the row's ticker override, warn about tickers that
    /// need a restart, drop state rows of tickers the new config removes, and park it until
    /// each ticker is flat.
    pub async fn set_pending(&mut self, version_id: i64, mut config: StrategyConfig) {
        self.spec.apply_to(&mut config);
        info!(
            book = %self.spec.name,
            version_id,
            open_positions = ?self.open_tickers(),
            "new config detected; applying to flat tickers now, others when they close"
        );
        // tickers added by the new config need a websocket subscription we don't have
        for t in &config.tickers {
            if !self.sessions.contains_key(t) {
                warn!(book = %self.spec.name, ticker = %t, version_id, "new config adds a ticker — requires a restart to subscribe; ignoring for now");
            }
        }
        // tickers removed by the new config: state rows go away when they are dropped
        if let Some(pool) = self.sink.pool() {
            for t in self.sessions.keys() {
                if !config.tickers.contains(t) {
                    let _ = delete_engine_state(pool, &self.spec.name, t).await;
                }
            }
        }
        self.pending = Some((version_id, config));
    }

    /// swap a flat ticker's engine to the pending config. `Some(true)` when the ticker
    /// was dropped (not in the new config), `Some(false)` when rebuilt, `None` when kept.
    fn apply_pending_to(&mut self, ticker: &str) -> Option<bool> {
        let (pending_id, pending_cfg) = self.pending.as_ref()?;
        let pending_id = *pending_id;
        if !pending_cfg.tickers.iter().any(|t| t == ticker) {
            info!(book = %self.spec.name, ticker, version_id = pending_id, "ticker not in new config, dropping session");
            self.sessions.remove(ticker);
            self.refresh_ticker_list();
            return Some(true);
        }
        match try_build_engine(pending_cfg, ticker, self.capital) {
            Some(engine) => {
                info!(book = %self.spec.name, ticker, version_id = pending_id, "engine rebuilt for new config");
                self.sessions
                    .insert(ticker.to_string(), LiveSession::new(engine, pending_id));
                Some(false)
            }
            None => {
                warn!(book = %self.spec.name, ticker, version_id = pending_id, "failed to rebuild engine, keeping old");
                None
            }
        }
    }

    /// apply the pending config to every flat ticker; clear it when nothing is left waiting.
    /// returns the tickers dropped from this book (the runtime prunes shared builders
    /// only when no book trades them any more).
    pub async fn apply_pending_where_flat(&mut self) -> Vec<String> {
        let Some((pending_id, _)) = self.pending.as_ref() else {
            return Vec::new();
        };
        let pending_id = *pending_id;
        let mut dropped = Vec::new();
        let tickers: Vec<String> = self.sessions.keys().cloned().collect();
        for t in tickers {
            let needs_swap = self
                .sessions
                .get(&t)
                .map(|s| s.config_version_id() != pending_id && !s.has_position())
                .unwrap_or(false);
            if needs_swap && self.apply_pending_to(&t) == Some(true) {
                dropped.push(t);
            }
        }
        let still_waiting = self
            .sessions
            .values()
            .any(|s| s.config_version_id() != pending_id);
        if !still_waiting {
            let (id, cfg) = self.pending.take().expect("pending checked above");
            info!(book = %self.spec.name, version_id = id, "config swap complete on all tickers");
            self.config = cfg;
            self.config_id = id;
        }
        dropped
    }

    // ── observability ──

    /// persist gate / near-miss diagnostics with dedupe and throttling (live sink only).
    async fn record_block_events(
        &mut self,
        ticker: &str,
        ts: DateTime<Utc>,
        result: &TickResult,
        last_price: f64,
        config_version_id: i64,
    ) {
        let Some(pool) = self.sink.pool() else { return };
        if let Some(reason) = &result.entry_blocked_by {
            let repeat = self
                .last_gate
                .get(ticker)
                .map(|(r, at)| r == reason && (ts - *at).num_seconds() < GATE_REPEAT_SECS)
                .unwrap_or(false);
            if !repeat {
                let ev = BlockEvent {
                    book: &self.spec.name,
                    ticker,
                    ts,
                    kind: BlockKind::Gate,
                    reason,
                    scores: &result.scores,
                    last_price,
                    config_version_id,
                };
                if let Err(e) = write_entry_block_event(pool, &ev).await {
                    warn!(book = %self.spec.name, error = %e, "failed to write entry block event");
                }
                self.last_gate
                    .insert(ticker.to_string(), (reason.clone(), ts));
            }
        } else if let Some(nm) = &result.near_miss {
            let throttled = self
                .last_near_miss
                .get(ticker)
                .map(|at| (ts - *at).num_seconds() < NEAR_MISS_THROTTLE_SECS)
                .unwrap_or(false);
            if !throttled {
                debug!(book = %self.spec.name, ticker, near_miss = %nm, "entry near-miss");
                let ev = BlockEvent {
                    book: &self.spec.name,
                    ticker,
                    ts,
                    kind: BlockKind::NearMiss,
                    reason: nm,
                    scores: &result.scores,
                    last_price,
                    config_version_id,
                };
                if let Err(e) = write_entry_block_event(pool, &ev).await {
                    warn!(book = %self.spec.name, error = %e, "failed to write near-miss event");
                }
                self.last_near_miss.insert(ticker.to_string(), ts);
            }
        }
    }

    pub fn state_row(&self, ticker: &str, shared: &TickerShared) -> Option<EngineStateRow> {
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
            book: self.spec.name.clone(),
            ticker: ticker.to_string(),
            last_bar_at: shared.last_bar_at,
            last_price: shared.last_price,
            scores: session.last_scores().clone(),
            position,
            daily_pnl: self.daily_pnl,
            ticker_realized_pnl: session.engine().cumulative_realized_pnl(),
            loss_breaker_active: session.engine().is_daily_loss_breaker_active(),
            entry_blocked_by: last.and_then(|r| r.entry_blocked_by.clone()),
            near_miss: last.and_then(|r| r.near_miss.clone()),
            feed_stale: shared.feed_stale,
            config_version_id: session.config_version_id(),
            pending_config_version_id: self.pending_id(),
            process_started_at: self.process_started_at,
            broker_mode: self.broker_mode.clone(),
        })
    }

    /// upsert the (book, ticker) state row (live sink only).
    pub async fn persist_state(&self, ticker: &str, shared: &TickerShared) {
        let Some(pool) = self.sink.pool() else { return };
        if let Some(row) = self.state_row(ticker, shared) {
            if let Err(e) = upsert_engine_state(pool, &row).await {
                warn!(book = %self.spec.name, ticker, error = %e, "failed to upsert engine_state");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use types::market::Timescale;
    use types::scoring::{AggregationMethod, ScoringConfig};
    use types::test_fixtures::make_indicator_config;

    fn spec(name: &str, role: BookRole, tickers: Option<&[&str]>) -> BookSpec {
        BookSpec {
            name: name.to_string(),
            role,
            config_version_id: None,
            tickers: tickers.map(|t| t.iter().map(|s| s.to_string()).collect()),
            capital: None,
            purpose: String::new(),
        }
    }

    /// a config that builds (rsi + score-threshold entry) for `tickers`.
    fn test_config(tickers: &[&str]) -> StrategyConfig {
        StrategyConfig {
            schema_version: "0.1".to_string(),
            config_id: 1,
            created_at: Utc::now(),
            created_by: "test".to_string(),
            parent_config_id: None,
            tickers: tickers.iter().map(|s| s.to_string()).collect(),
            indicators: vec![make_indicator_config(
                "rsi",
                "rsi_5m",
                Timescale::FiveMinute,
                1.0,
                vec![("period", json!(14))],
            )],
            actions: vec![types::ActionConfig {
                action_type: "score_threshold_entry".to_string(),
                instance_id: "e1".to_string(),
                phase: types::ActionPhase::Entry,
                enabled: true,
                priority: 0,
                params: vec![("entry_threshold", json!(0.5))]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
                last_modified_by: None,
                last_modified_at: None,
                modification_reason: None,
            }],
            scoring: ScoringConfig {
                timescale_weights: vec![(Timescale::FiveMinute, 1.0)].into_iter().collect(),
                entry_threshold: 0.5,
                exit_threshold: -0.3,
                aggregation: AggregationMethod::WeightedSum,
                hard_gate_timescales: vec![],
                agreement: None,
                dynamic_fusion: None,
                hard_gate_indicators: HashMap::new(),
                hourly_exit_override: None,
            },
            session: types::config::SessionConfig {
                no_new_entries_after: "15:30".to_string(),
                force_exit_by: "15:55".to_string(),
                avoid_first_minutes: 5,
                max_concurrent_positions: 3,
                max_capital_deployed_pct: 0.15,
                entry_cooldown_ms: 0,
                max_daily_loss_pct: None,
                max_position_pct: None,
                warmup_days: None,
            },
            ticker_overrides: HashMap::new(),
        }
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn warmup_primary_only_keeps_eight_days() {
        let names = strings(&["AAPL", "AMZN", "MSFT", "NVDA"]);
        let books = [WarmupInput { book: "primary", is_primary: true, tickers: &names, warmup_days: None }];
        let plan = warmup_plan(&books, "SPY");
        for t in &names {
            assert_eq!(plan.days_for(t), 8);
        }
        assert_eq!(plan.days_for("SPY"), 8);
        assert_eq!(plan.days.len(), 5);
        assert!(plan.raised.is_empty());
    }

    #[test]
    fn warmup_qqq_book_raises_only_qqq_and_the_index() {
        let names = strings(&["AAPL", "AMZN", "MSFT", "NVDA"]);
        let qqq = strings(&["QQQ"]);
        let books = [
            WarmupInput { book: "primary", is_primary: true, tickers: &names, warmup_days: None },
            WarmupInput { book: "qqq-noise-pm-vol", is_primary: false, tickers: &qqq, warmup_days: Some(22) },
        ];
        let plan = warmup_plan(&books, "SPY");
        assert_eq!(plan.days_for("QQQ"), 22);
        for t in &names {
            assert_eq!(plan.days_for(t), 8, "{t} keeps the primary's 8 days");
        }
        // SPY is not traded: max over all books (it only seeds the cross tracker)
        assert_eq!(plan.days_for("SPY"), 22);
        assert!(plan.raised.is_empty(), "QQQ is not a primary name: no parity warning");
    }

    #[test]
    fn warmup_book_on_a_primary_name_raises_it_with_a_parity_warning() {
        let names = strings(&["AAPL", "AMZN", "MSFT", "NVDA"]);
        let aapl = strings(&["AAPL"]);
        let short = strings(&["NVDA"]);
        let books = [
            WarmupInput { book: "primary", is_primary: true, tickers: &names, warmup_days: None },
            WarmupInput { book: "aapl-30", is_primary: false, tickers: &aapl, warmup_days: Some(30) },
            // asking for less than the primary's 8 never shortens a shared window
            WarmupInput { book: "nvda-3", is_primary: false, tickers: &short, warmup_days: Some(3) },
        ];
        let plan = warmup_plan(&books, "SPY");
        assert_eq!(plan.days_for("AAPL"), 30);
        assert_eq!(plan.days_for("NVDA"), 8);
        assert_eq!(plan.days_for("MSFT"), 8);
        assert_eq!(
            plan.raised,
            vec![WarmupRaise { symbol: "AAPL".into(), days: 30, book: "aapl-30".into() }]
        );
    }

    #[test]
    fn warmup_traded_index_follows_the_traded_rule() {
        let names = strings(&["AAPL"]);
        let spy = strings(&["SPY"]);
        let books = [
            WarmupInput { book: "primary", is_primary: true, tickers: &names, warmup_days: Some(30) },
            WarmupInput { book: "spy", is_primary: false, tickers: &spy, warmup_days: Some(12) },
        ];
        let plan = warmup_plan(&books, "SPY");
        assert_eq!(plan.days_for("SPY"), 12, "the SPY builder follows the books trading SPY");
        assert_eq!(plan.days_for("AAPL"), 30);
        // the primary's own config raised its name: still flagged (8-day replays no longer match)
        assert_eq!(plan.raised.len(), 1);
        assert_eq!(plan.raised[0].book, "primary");
    }

    fn replay_sink() -> Sink {
        Sink::Replay(Arc::new(Mutex::new(Vec::new())))
    }

    #[test]
    fn roster_requires_exactly_one_primary() {
        let none = vec![spec("a", BookRole::Shadow, None)];
        assert!(matches!(validate_roster(none), Err(BookError::NoPrimary)));

        let two = vec![
            spec("primary", BookRole::Primary, None),
            spec("p2", BookRole::Primary, None),
        ];
        assert!(matches!(validate_roster(two), Err(BookError::MultiplePrimaries(_))));

        // shadows listed before the primary: the primary comes out first, shadows keep order
        let ok = validate_roster(vec![
            spec("s1", BookRole::Shadow, None),
            spec("primary", BookRole::Primary, None),
            spec("s2", BookRole::Shadow, None),
        ])
        .unwrap();
        let names: Vec<&str> = ok.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["primary", "s1", "s2"]);
    }

    #[test]
    fn effective_tickers_prefer_the_row_override() {
        let cfg = test_config(&["AAPL", "MSFT"]);
        let own = spec("s", BookRole::Shadow, None);
        assert_eq!(own.effective_tickers(&cfg), vec!["AAPL", "MSFT"]);
        let empty = spec("s", BookRole::Shadow, Some(&[]));
        assert_eq!(empty.effective_tickers(&cfg), vec!["AAPL", "MSFT"]);
        let over = spec("s", BookRole::Shadow, Some(&["AMD"]));
        assert_eq!(over.effective_tickers(&cfg), vec!["AMD"]);
        let plan = BookPlan::new(over, 7, cfg);
        assert_eq!(plan.config.tickers, vec!["AMD"]);
    }

    #[test]
    fn limits_skip_excess_shadows_but_never_the_primary() {
        let limits = Limits { max_books: 2, max_symbols: 4 };
        let plans = vec![
            BookPlan::new(spec("primary", BookRole::Primary, None), 12, test_config(&["AAPL", "MSFT"])),
            // AAPL, MSFT, SPY + AMD = 4 symbols: fits
            BookPlan::new(spec("s1", BookRole::Shadow, Some(&["AMD"])), 12, test_config(&["AAPL"])),
            // book count would be 3 > 2
            BookPlan::new(spec("s2", BookRole::Shadow, None), 12, test_config(&["AAPL"])),
        ];
        let (kept, skipped) = apply_limits(plans, &limits, "SPY");
        assert_eq!(kept.iter().map(|p| p.spec.name.as_str()).collect::<Vec<_>>(), vec!["primary", "s1"]);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0].0.spec.name, "s2");
        assert!(skipped[0].1.contains("MAX_BOOKS"));

        // symbol cap: a primary with 5 tickers is still kept even though 6 > 4
        let limits = Limits { max_books: 8, max_symbols: 4 };
        let plans = vec![
            BookPlan::new(spec("primary", BookRole::Primary, None), 12, test_config(&["A", "B", "C", "D", "E"])),
            BookPlan::new(spec("s1", BookRole::Shadow, Some(&["F"])), 12, test_config(&["A"])),
            BookPlan::new(spec("s2", BookRole::Shadow, Some(&["A"])), 12, test_config(&["A"])),
        ];
        let (kept, skipped) = apply_limits(plans, &limits, "SPY");
        // s2 adds no new symbol, s1 would add one
        assert_eq!(kept.iter().map(|p| p.spec.name.as_str()).collect::<Vec<_>>(), vec!["primary", "s2"]);
        assert_eq!(skipped[0].0.spec.name, "s1");
        assert!(skipped[0].1.contains("MAX_SYMBOLS"));
    }

    #[test]
    fn shadow_always_gets_a_simulated_broker() {
        // the environment says alpaca; a shadow has no way to receive that broker
        std::env::set_var("BROKER_MODE", "alpaca_paper");
        let plan = BookPlan::new(spec("shadow:x", BookRole::Shadow, None), 10, test_config(&["SPY"]));
        let book = Book::new_shadow(plan, 10_000.0, replay_sink(), SHADOW_SLIPPAGE_BPS, Utc::now()).unwrap();
        assert_eq!(book.broker_kind(), "simulated");
        assert_eq!(book.broker_mode(), "simulated");
        assert!(!book.is_primary());
        assert_eq!(book.tickers(), &["SPY".to_string()]);
    }

    #[test]
    fn primary_keeps_the_broker_it_is_given() {
        let plan = BookPlan::new(spec("primary", BookRole::Primary, None), 12, test_config(&["SPY", "QQQ"]));
        let book = Book::new_primary(plan, Box::new(SimulatedBroker::new(5.0)), 10_000.0, replay_sink(), Utc::now()).unwrap();
        assert!(book.is_primary());
        assert!(book.follows_promoted());
        assert_eq!(book.tickers(), &["QQQ".to_string(), "SPY".to_string()]);
        assert_eq!(book.config_id(), 12);
    }

    #[test]
    fn book_capital_override_and_no_engine_error() {
        let mut s = spec("s", BookRole::Shadow, None);
        s.capital = Some(2_500.0);
        let plan = BookPlan::new(s, 10, test_config(&["SPY"]));
        let book = Book::new_shadow(plan, 10_000.0, replay_sink(), 0.0, Utc::now()).unwrap();
        assert!((book.capital() - 2_500.0).abs() < f64::EPSILON);

        let mut bad = test_config(&["SPY"]);
        bad.indicators[0].indicator_type = "nonexistent".to_string();
        let plan = BookPlan::new(spec("s", BookRole::Shadow, None), 10, bad);
        assert!(matches!(
            Book::new_shadow(plan, 10_000.0, replay_sink(), 0.0, Utc::now()),
            Err(BookError::NoEngines(_))
        ));
    }

    #[tokio::test]
    async fn follow_promoted_reapplies_the_ticker_override_on_reload() {
        let over = spec("shadow:amd", BookRole::Shadow, Some(&["SPY"]));
        let plan = BookPlan::new(over, 12, test_config(&["AAPL", "MSFT"]));
        let mut book = Book::new_shadow(plan, 10_000.0, replay_sink(), 0.0, Utc::now()).unwrap();
        assert_eq!(book.tickers(), &["SPY".to_string()]);

        // a new promoted row with a different ticker list arrives
        book.set_pending(13, test_config(&["AAPL", "MSFT", "NVDA"])).await;
        assert_eq!(book.pending_id(), Some(13));
        assert_eq!(book.pending_config().unwrap().tickers, vec!["SPY"]);

        // flat → swapped straight away, still on the row's ticker set
        let dropped = book.apply_pending_where_flat().await;
        assert!(dropped.is_empty());
        assert_eq!(book.config_id(), 13);
        assert!(!book.has_pending());
        assert_eq!(book.config().tickers, vec!["SPY"]);
        assert_eq!(book.tickers(), &["SPY".to_string()]);
        assert_eq!(book.session("SPY").unwrap().config_version_id(), 13);
    }

    #[tokio::test]
    async fn reload_drops_tickers_the_new_config_removes() {
        let plan = BookPlan::new(spec("primary", BookRole::Primary, None), 12, test_config(&["SPY", "QQQ"]));
        let mut book = Book::new_primary(plan, Box::new(SimulatedBroker::new(0.0)), 10_000.0, replay_sink(), Utc::now()).unwrap();
        book.set_pending(13, test_config(&["SPY"])).await;
        let dropped = book.apply_pending_where_flat().await;
        assert_eq!(dropped, vec!["QQQ".to_string()]);
        assert_eq!(book.tickers(), &["SPY".to_string()]);
        assert!(!book.trades("QQQ"));
        assert_eq!(book.config_id(), 13);
    }

    #[tokio::test]
    async fn on_bar_ticks_only_hosted_tickers_and_reports_state() {
        use types::test_fixtures::{make_market_state_ohlcv, ranging_ohlcv};
        let plan = BookPlan::new(spec("shadow:x", BookRole::Shadow, None), 10, test_config(&["SPY"]));
        let mut book = Book::new_shadow(plan, 10_000.0, replay_sink(), 0.0, Utc::now()).unwrap();
        let data = ranging_ohlcv(5, 100.0, 0.5);
        let ms = make_market_state_ohlcv(Timescale::FiveMinute, &data);
        let shared = TickerShared { last_bar_at: Some(ms.timestamp), last_price: Some(ms.last_price), feed_stale: false };
        assert!(book.on_bar("QQQ", ms.clone(), &shared).await.is_none());
        let out = book.on_bar("SPY", ms, &shared).await.expect("hosted ticker ticks");
        assert!(out.trade.is_none());
        assert!(out.dropped.is_empty());
        let row = book.state_row("SPY", &shared).unwrap();
        assert_eq!(row.book, "shadow:x");
        assert_eq!(row.config_version_id, 10);
        assert_eq!(row.broker_mode, "simulated");
        assert!(book.state_row("QQQ", &shared).is_none());
    }
}
