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

use chrono::{DateTime, Duration, NaiveDate, Utc};
use tracing::{debug, error, info, warn};
use types::action::ExitReason;
use types::config::StrategyConfig;
use types::market::MarketState;
use types::tick_result::{TickEvent, TickResult};

use crate::broker::{Broker, BrokerError, BrokerPosition, OrderFill, OrderState, OrphanClose, SimulatedBroker};
use crate::config_watcher::try_build_engine;
use crate::live_session::{LiveSession, TradeWithScores};
use crate::notify::{notify_critical, notify_warning};
use crate::session_clock::{eastern_date, is_regular_hours};
use types::scoring::TimescaleScores;
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
/// after a failed / unconfirmed entry order a ticker takes no new entry for at least this
/// long (or the config's `entry_cooldown_ms` when that is longer), so one broker fault
/// cannot re-fire on every bar.
pub const ORDER_FAIL_COOLDOWN_MS: i64 = 5 * 60 * 1000;
/// a close the broker has not confirmed for this long gets the human a CRITICAL (once).
pub const EXIT_PENDING_ALERT_SECS: i64 = 5 * 60;
/// how often the primary reconciles its engines against the broker during regular hours.
pub const LIVE_RECONCILE_SECS: i64 = 5 * 60;
/// a broker ↔ engine mismatch must be seen on this many consecutive reconciliation
/// checks before anything is done about it (an order can be in flight).
pub const MISMATCH_SIGHTINGS: u32 = 2;

/// what to do with a position the primary's broker holds that no engine knows about
/// (`ORPHAN_POLICY`): `adopt` (default) adopts tickers the primary trades and closes the
/// rest; `close` closes everything; `ignore` only logs (today's pre-fix behaviour).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrphanPolicy {
    Adopt,
    Close,
    Ignore,
}

impl OrphanPolicy {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "adopt" => Some(OrphanPolicy::Adopt),
            "close" => Some(OrphanPolicy::Close),
            "ignore" => Some(OrphanPolicy::Ignore),
            _ => None,
        }
    }

    pub fn from_env() -> Self {
        match std::env::var("ORPHAN_POLICY") {
            Ok(v) => Self::parse(&v).unwrap_or_else(|| {
                warn!(value = %v, "ORPHAN_POLICY not one of adopt|close|ignore — using adopt");
                OrphanPolicy::Adopt
            }),
            Err(_) => OrphanPolicy::Adopt,
        }
    }
}

/// what startup reconciliation did with one broker position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrphanAction {
    Adopted,
    Closed,
    /// an after-hours limit is resting at the broker.
    CloseWorking,
    CloseFailed(String),
    Ignored,
}

#[derive(Debug, Clone)]
pub struct OrphanOutcome {
    pub ticker: String,
    pub action: OrphanAction,
}

/// an exit the engine decided on but the broker has not confirmed: the engine keeps the
/// position, no trade is recorded, and the order (or the broker's positions) is
/// re-checked on every bar and heartbeat until it fills, the broker turns out flat, or
/// the order ends and a new close is submitted.
#[derive(Debug, Clone)]
pub struct ExitPending {
    /// the working close order; `None` when the close request itself errored (the
    /// broker's positions decide).
    pub order_id: Option<String>,
    pub reason: ExitReason,
    /// the engine's exit price at the signal (fallback when no fill price is known).
    pub engine_exit_price: f64,
    pub since: DateTime<Utc>,
    pub checks: u32,
    /// the one CRITICAL after `EXIT_PENDING_ALERT_SECS` has been sent.
    pub alerted: bool,
}

/// how a close request at the broker ended, from the book's point of view.
enum CloseOutcome {
    Filled(OrderFill),
    /// the broker holds nothing in the ticker (someone else closed it).
    Flat,
    /// the broker may still hold it: a working order (`Some(id)`) or an errored request.
    Pending(Option<String>, String),
}

/// what a pending-exit check established.
enum ExitResolution {
    Filled { price: Option<f64> },
    BrokerFlat { price: Option<f64> },
    /// the order ended (cancelled / expired / rejected) and the broker still holds the
    /// position: submit a new close.
    Resubmit,
    Still,
}

/// an entry whose broker order failed or could not be confirmed: the engine is flat,
/// the broker may not be. re-checked on the ticker's next bars until resolved.
#[derive(Debug, Clone)]
struct Unreconciled {
    scores: Option<TimescaleScores>,
    reason: Option<String>,
    since: DateTime<Utc>,
    attempts: u32,
}

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
    /// primary only: tickers whose failed entry order still needs the broker's word.
    unreconciled: HashMap<String, Unreconciled>,
    /// tickers whose exit the broker has not confirmed (engine still in the position).
    exit_pending: HashMap<String, ExitPending>,
    /// live reconciliation debounce: consecutive checks a broker position was unknown
    /// to the engine / an engine position was missing at the broker.
    orphan_sightings: HashMap<String, u32>,
    flat_sightings: HashMap<String, u32>,
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
            unreconciled: HashMap::new(),
            exit_pending: HashMap::new(),
            orphan_sightings: HashMap::new(),
            flat_sightings: HashMap::new(),
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

    pub fn session_mut(&mut self, ticker: &str) -> Option<&mut LiveSession> {
        self.sessions.get_mut(ticker)
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

    /// tickers whose failed order is still unreconciled against the broker (primary only).
    pub fn unreconciled_tickers(&self) -> Vec<String> {
        let mut v: Vec<String> = self.unreconciled.keys().cloned().collect();
        v.sort();
        v
    }

    /// tickers whose exit the broker has not confirmed yet (sorted).
    pub fn exit_pending_tickers(&self) -> Vec<String> {
        let mut v: Vec<String> = self.exit_pending.keys().cloned().collect();
        v.sort();
        v
    }

    pub fn exit_pending(&self, ticker: &str) -> Option<&ExitPending> {
        self.exit_pending.get(ticker)
    }

    pub fn has_exit_pending(&self) -> bool {
        !self.exit_pending.is_empty()
    }

    // ── broker ↔ engine invariant ──

    /// the broker rejected — or could not confirm — the entry order: roll the engine
    /// back, gate the ticker's entries for the cooldown, and (primary) make sure the
    /// broker does not hold a position the engine just forgot.
    pub async fn handle_failed_entry(&mut self, ticker: &str, bar_ts: DateTime<Utc>, error: &BrokerError) {
        error!(book = %self.spec.name, ticker, error = %error, "broker order failed, undoing position");
        let cooldown_ms = self.config.session.entry_cooldown_ms.max(ORDER_FAIL_COOLDOWN_MS);
        let until = bar_ts + Duration::milliseconds(cooldown_ms);
        let stash = match self.sessions.get_mut(ticker) {
            Some(s) => {
                let stash = s.take_entry_stash();
                s.undo_open();
                s.suppress_entries_until(until);
                stash
            }
            None => (None, None),
        };
        warn!(book = %self.spec.name, ticker, until = %until, "entries suppressed after the failed order");
        if !self.is_primary() {
            return;
        }
        self.unreconciled.insert(
            ticker.to_string(),
            Unreconciled { scores: stash.0, reason: stash.1, since: bar_ts, attempts: 0 },
        );
        self.reconcile_ticker(ticker, bar_ts).await;
    }

    /// primary: ask the broker whether it holds `ticker`; adopt it if so, clear the
    /// entry if it is flat, keep asking (next bar) if it cannot be reached.
    async fn reconcile_ticker(&mut self, ticker: &str, now: DateTime<Utc>) {
        let Some(mut pending) = self.unreconciled.remove(ticker) else { return };
        pending.attempts += 1;
        match self.broker.open_positions().await {
            Ok(list) => match list.into_iter().find(|p| p.ticker == ticker) {
                Some(p) => {
                    let reason = match &pending.reason {
                        Some(r) if !r.is_empty() => format!("adopted:failed-order ({r})"),
                        _ => "adopted:failed-order".to_string(),
                    };
                    let adopted = self.adopt(&p, now, pending.scores.clone(), &reason);
                    let msg = format!(
                        "CRITICAL {} {}: broker holds {:?} {} @ {:.4} after a failed entry order (since {}) — {}",
                        self.spec.name,
                        ticker,
                        p.direction,
                        p.quantity,
                        p.entry_price,
                        pending.since.format("%H:%M:%SZ"),
                        if adopted { "adopted into the engine; the exit stack manages it" } else { "ADOPTION FAILED, close it by hand" }
                    );
                    notify_critical(msg);
                }
                None => {
                    info!(book = %self.spec.name, ticker, attempts = pending.attempts, "broker confirms flat after the failed order");
                }
            },
            Err(e) => {
                error!(book = %self.spec.name, ticker, attempts = pending.attempts, error = %e, "broker unreachable — cannot confirm what it holds; retrying on the next bar");
                if pending.attempts == 1 {
                    notify_critical(format!(
                        "CRITICAL {} {}: entry order failed AND the broker cannot be reached ({e}) — it may hold a position the engine does not; retrying every bar",
                        self.spec.name, ticker
                    ));
                }
                self.unreconciled.insert(ticker.to_string(), pending);
            }
        }
    }

    /// force the broker's position into the ticker's engine. false when the engine
    /// refused (already in a position / bad numbers).
    fn adopt(&mut self, p: &BrokerPosition, now: DateTime<Utc>, scores: Option<TimescaleScores>, reason: &str) -> bool {
        let Some(session) = self.sessions.get_mut(&p.ticker) else {
            error!(book = %self.spec.name, ticker = %p.ticker, "cannot adopt: no session for the ticker");
            return false;
        };
        match session.adopt_broker_position(&p.ticker, p.direction, p.quantity, p.entry_price, now, scores, reason) {
            Ok(()) => {
                warn!(book = %self.spec.name, ticker = %p.ticker, direction = ?p.direction, quantity = p.quantity, entry_price = p.entry_price, reason, "broker position adopted into the engine");
                true
            }
            Err(e) => {
                error!(book = %self.spec.name, ticker = %p.ticker, error = %e, "adoption refused by the engine");
                false
            }
        }
    }

    /// one position the broker holds that no engine knows about: adopt / close / ignore
    /// per `policy`, and tell the human. `context` names the check ("startup" / "intraday").
    async fn handle_orphan(&mut self, p: &BrokerPosition, policy: OrphanPolicy, now: DateTime<Utc>, context: &str) -> OrphanAction {
        let traded = self.sessions.contains_key(&p.ticker);
        error!(
            book = %self.spec.name,
            ticker = %p.ticker,
            direction = ?p.direction,
            quantity = p.quantity,
            entry_price = p.entry_price,
            traded,
            ?policy,
            context,
            "BROKER HOLDS A POSITION THE ENGINE DOES NOT KNOW ABOUT"
        );
        let reason = format!("adopted:orphan-at-{context}");
        let action = match (policy, traded) {
            (OrphanPolicy::Ignore, _) => OrphanAction::Ignored,
            (OrphanPolicy::Adopt, true) => {
                if self.adopt(p, now, None, &reason) {
                    OrphanAction::Adopted
                } else {
                    OrphanAction::CloseFailed("adoption refused".to_string())
                }
            }
            (OrphanPolicy::Adopt, false) | (OrphanPolicy::Close, _) => {
                match self.broker.close_orphan(p, is_regular_hours(now)).await {
                    Ok(OrphanClose::Filled(f)) => {
                        warn!(book = %self.spec.name, ticker = %p.ticker, fill_price = f.fill_price, quantity = f.quantity, "orphan closed at the broker");
                        OrphanAction::Closed
                    }
                    Ok(OrphanClose::Working { order_id, limit_price }) => {
                        warn!(book = %self.spec.name, ticker = %p.ticker, order_id, limit_price, "orphan close resting as an extended-hours limit");
                        OrphanAction::CloseWorking
                    }
                    Err(BrokerError::Working { order_id }) => {
                        warn!(book = %self.spec.name, ticker = %p.ticker, order_id, "orphan close still working at the broker (not cancelled)");
                        OrphanAction::CloseWorking
                    }
                    Err(e) => {
                        error!(book = %self.spec.name, ticker = %p.ticker, error = %e, "orphan close FAILED");
                        OrphanAction::CloseFailed(e.to_string())
                    }
                }
            }
        };
        let what = match &action {
            OrphanAction::Adopted => "adopted into the engine (the exit stack closes it today)".to_string(),
            OrphanAction::Closed => "closed at the broker".to_string(),
            OrphanAction::CloseWorking => "close order working at the broker (left working, re-checked)".to_string(),
            OrphanAction::CloseFailed(e) => format!("CLOSE FAILED ({e}) — close it by hand"),
            OrphanAction::Ignored => "IGNORED per ORPHAN_POLICY=ignore — close it by hand".to_string(),
        };
        notify_critical(format!(
            "CRITICAL {} {context}: broker holds {:?} {} {} @ {:.4} the engine does not know — {}",
            self.spec.name, p.direction, p.quantity, p.ticker, p.entry_price, what
        ));
        action
    }

    /// startup reconciliation (primary only — shadows are simulated and start empty):
    /// every position the broker holds that no engine knows about is adopted, closed or
    /// ignored per `policy`, and the human is told either way.
    pub async fn reconcile_orphans(&mut self, policy: OrphanPolicy, now: DateTime<Utc>) -> Result<Vec<OrphanOutcome>, BrokerError> {
        let open = self.broker.open_positions().await?;
        let mut outcomes = Vec::new();
        for p in open {
            let known = self.sessions.get(&p.ticker).map(|s| s.has_position()).unwrap_or(false);
            if known {
                continue;
            }
            let action = self.handle_orphan(&p, policy, now, "start").await;
            outcomes.push(OrphanOutcome { ticker: p.ticker.clone(), action });
        }
        Ok(outcomes)
    }

    /// intraday reconciliation (primary, every `LIVE_RECONCILE_SECS` during regular
    /// hours): the startup orphan policy for broker positions no engine knows about, and
    /// the reverse — an engine position the broker does not hold is marked closed at the
    /// last price (`ManualOverride`, "broker-flat"). either mismatch must be seen on
    /// `MISMATCH_SIGHTINGS` consecutive checks before anything is done (an order can be
    /// in flight); tickers with an exit pending or an unreconciled entry are skipped.
    /// returns (orphans handled, tickers marked broker-flat).
    pub async fn reconcile_live(
        &mut self,
        policy: OrphanPolicy,
        now: DateTime<Utc>,
        last_prices: &HashMap<String, f64>,
    ) -> Result<(Vec<OrphanOutcome>, Vec<String>), BrokerError> {
        let held = self.broker.open_positions().await?;
        let in_flight = |b: &Self, t: &str| b.exit_pending.contains_key(t) || b.unreconciled.contains_key(t);

        // broker → engine: positions no engine knows about
        let mut orphans = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for p in &held {
            let known = self.sessions.get(&p.ticker).map(|s| s.has_position()).unwrap_or(false);
            if known || in_flight(self, &p.ticker) {
                continue;
            }
            seen.insert(p.ticker.clone());
            let n = self.orphan_sightings.entry(p.ticker.clone()).or_insert(0);
            *n += 1;
            let n = *n;
            if n < MISMATCH_SIGHTINGS {
                warn!(book = %self.spec.name, ticker = %p.ticker, direction = ?p.direction, quantity = p.quantity, sighting = n, "broker holds a position the engine does not — waiting for a second sighting");
                continue;
            }
            self.orphan_sightings.remove(&p.ticker);
            let action = self.handle_orphan(p, policy, now, "intraday").await;
            orphans.push(OrphanOutcome { ticker: p.ticker.clone(), action });
        }
        self.orphan_sightings.retain(|t, _| seen.contains(t));

        // engine → broker: positions the broker does not hold
        let mut flat = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for ticker in self.open_tickers() {
            if in_flight(self, &ticker) || held.iter().any(|p| p.ticker == ticker) {
                continue;
            }
            seen.insert(ticker.clone());
            let n = self.flat_sightings.entry(ticker.clone()).or_insert(0);
            *n += 1;
            let n = *n;
            if n < MISMATCH_SIGHTINGS {
                warn!(book = %self.spec.name, ticker = %ticker, sighting = n, "engine holds a position the broker does not — waiting for a second sighting");
                continue;
            }
            self.flat_sightings.remove(&ticker);
            let price = last_prices.get(&ticker).copied().unwrap_or(0.0);
            let closed = self
                .sessions
                .get_mut(&ticker)
                .and_then(|s| s.force_close(price, now, ExitReason::ManualOverride));
            match closed {
                Some(tws) => {
                    error!(book = %self.spec.name, ticker = %ticker, price, pnl = format!("{:.2}", tws.trade.pnl), "broker-flat: engine position the broker does not hold — marked closed at the last price");
                    self.daily_pnl += tws.trade.pnl;
                    self.record_trade(&tws, None).await;
                    notify_warning(format!(
                        "WARNING {} {ticker}: broker-flat — the engine held {:?} {} @ {:.4} the broker does not; recorded closed at {:.4} (ManualOverride), pnl {:.2}",
                        self.spec.name, tws.trade.direction, tws.trade.size, tws.trade.entry_price, price, tws.trade.pnl
                    ));
                    flat.push(ticker);
                }
                None => warn!(book = %self.spec.name, ticker = %ticker, "broker-flat: force_close produced no trade"),
            }
        }
        self.flat_sightings.retain(|t, _| seen.contains(t));
        if !flat.is_empty() && self.pending.is_some() {
            let _ = self.apply_pending_where_flat().await;
        }
        Ok((orphans, flat))
    }

    // ── exits the broker has not confirmed ──

    /// ask the broker to close `ticker` and classify the answer.
    async fn close_at_broker(&self, ticker: &str) -> CloseOutcome {
        match self.broker.close_position(ticker).await {
            Ok(fill) => CloseOutcome::Filled(fill),
            Err(BrokerError::NoPosition(_)) => CloseOutcome::Flat,
            Err(BrokerError::Working { order_id }) => CloseOutcome::Pending(Some(order_id), "still working at the deadline".into()),
            Err(e) => CloseOutcome::Pending(None, e.to_string()),
        }
    }

    /// the broker did not confirm the close: keep the engine in the position (no trade
    /// is recorded) and track the order until it resolves.
    fn begin_exit_pending(&mut self, ticker: &str, order_id: Option<String>, reason: ExitReason, engine_exit_price: f64, now: DateTime<Utc>, why: &str) {
        error!(book = %self.spec.name, ticker, ?order_id, ?reason, why, "broker close NOT confirmed — engine keeps the position, exit pending (re-checked every bar / heartbeat)");
        self.exit_pending.insert(
            ticker.to_string(),
            ExitPending { order_id: order_id.clone(), reason: reason.clone(), engine_exit_price, since: now, checks: 0, alerted: false },
        );
        if self.is_primary() {
            notify_warning(format!(
                "WARNING {} {ticker}: close ({reason:?}) not confirmed by the broker ({why}; order {}) — position kept open in the engine, re-checking until it fills",
                self.spec.name,
                order_id.as_deref().unwrap_or("none")
            ));
        }
    }

    /// record the exit of a pending ticker: engine close at `price` with the pending
    /// reason, broker fill price when known.
    async fn finish_exit_pending(&mut self, ticker: &str, p: &ExitPending, price: f64, broker_exit: Option<f64>, now: DateTime<Utc>, how: &str) {
        let closed = self
            .sessions
            .get_mut(ticker)
            .and_then(|s| s.force_close(price, now, p.reason.clone()));
        match closed {
            Some(tws) => {
                info!(book = %self.spec.name, ticker, price, ?broker_exit, reason = ?p.reason, checks = p.checks, pending_for_s = (now - p.since).num_seconds(), how, "pending exit resolved");
                self.daily_pnl += tws.trade.pnl;
                self.record_trade(&tws, broker_exit).await;
            }
            None => warn!(book = %self.spec.name, ticker, how, "pending exit resolved but the engine had no position"),
        }
        if self.pending.is_some() {
            let _ = self.apply_pending_where_flat().await;
        }
    }

    /// what the broker says about a pending exit now.
    async fn resolve_exit_pending(&self, ticker: &str, p: &ExitPending) -> ExitResolution {
        let held_now = |list: &[BrokerPosition]| list.iter().any(|h| h.ticker == ticker);
        let Some(order_id) = &p.order_id else {
            return match self.broker.open_positions().await {
                Ok(list) if held_now(&list) => ExitResolution::Resubmit,
                Ok(_) => ExitResolution::BrokerFlat { price: None },
                Err(e) => {
                    warn!(book = %self.spec.name, ticker, error = %e, "pending exit: broker unreachable");
                    ExitResolution::Still
                }
            };
        };
        match self.broker.order_status(order_id).await {
            Ok(view) => match view.state {
                OrderState::Filled => ExitResolution::Filled { price: view.avg_fill_price },
                OrderState::Working => {
                    debug!(book = %self.spec.name, ticker, order_id, filled_qty = view.filled_qty, "pending exit: order still working");
                    ExitResolution::Still
                }
                OrderState::Terminal(status) => match self.broker.open_positions().await {
                    Ok(list) if held_now(&list) => {
                        warn!(book = %self.spec.name, ticker, order_id, status, filled_qty = view.filled_qty, "pending exit: order ended but the broker still holds the position — re-submitting the close");
                        ExitResolution::Resubmit
                    }
                    Ok(_) => ExitResolution::BrokerFlat { price: view.avg_fill_price.filter(|_| view.filled_qty > 0.0) },
                    Err(e) => {
                        warn!(book = %self.spec.name, ticker, order_id, status, error = %e, "pending exit: order ended, positions unreachable");
                        ExitResolution::Still
                    }
                },
            },
            Err(e) => {
                warn!(book = %self.spec.name, ticker, order_id, error = %e, "pending exit: order lookup failed — asking the positions list");
                match self.broker.open_positions().await {
                    Ok(list) if held_now(&list) => ExitResolution::Still,
                    Ok(_) => ExitResolution::BrokerFlat { price: None },
                    Err(_) => ExitResolution::Still,
                }
            }
        }
    }

    /// re-check one pending exit. true when it resolved (the ticker is flat now).
    pub async fn check_exit_pending(&mut self, ticker: &str, now: DateTime<Utc>, last_price: Option<f64>) -> bool {
        let Some(mut p) = self.exit_pending.remove(ticker) else { return false };
        p.checks += 1;
        let fallback = last_price.filter(|x| *x > 0.0).unwrap_or(p.engine_exit_price);
        let resolution = self.resolve_exit_pending(ticker, &p).await;
        match resolution {
            ExitResolution::Filled { price } => {
                let px = price.filter(|x| *x > 0.0).unwrap_or(fallback);
                self.finish_exit_pending(ticker, &p, px, price.filter(|x| *x > 0.0), now, "order filled").await;
                true
            }
            ExitResolution::BrokerFlat { price } => {
                let px = price.filter(|x| *x > 0.0).unwrap_or(fallback);
                warn!(book = %self.spec.name, ticker, price = px, fill_known = price.is_some(), "pending exit: broker is flat — recording the exit");
                if self.is_primary() {
                    notify_warning(format!(
                        "WARNING {} {ticker}: broker flat while a close was pending — exit recorded at {px:.4} ({})",
                        self.spec.name,
                        if price.is_some() { "order's fill price" } else { "last known price; fill unknown" }
                    ));
                }
                self.finish_exit_pending(ticker, &p, px, price.filter(|x| *x > 0.0), now, "broker flat").await;
                true
            }
            ExitResolution::Resubmit => match self.close_at_broker(ticker).await {
                CloseOutcome::Filled(f) => {
                    let px = Some(f.fill_price).filter(|x| *x > 0.0);
                    self.finish_exit_pending(ticker, &p, px.unwrap_or(fallback), px, now, "re-submitted close filled").await;
                    true
                }
                CloseOutcome::Flat => {
                    self.finish_exit_pending(ticker, &p, fallback, None, now, "broker flat on re-submission").await;
                    true
                }
                CloseOutcome::Pending(order_id, why) => {
                    warn!(book = %self.spec.name, ticker, ?order_id, why, "pending exit: re-submitted close not confirmed either");
                    p.order_id = order_id;
                    self.keep_exit_pending(ticker, p, now);
                    false
                }
            },
            ExitResolution::Still => {
                self.keep_exit_pending(ticker, p, now);
                false
            }
        }
    }

    fn keep_exit_pending(&mut self, ticker: &str, mut p: ExitPending, now: DateTime<Utc>) {
        let age = (now - p.since).num_seconds();
        if !p.alerted && age >= EXIT_PENDING_ALERT_SECS {
            p.alerted = true;
            error!(book = %self.spec.name, ticker, order_id = ?p.order_id, age_s = age, checks = p.checks, "EXIT STILL PENDING — the broker has not confirmed the close");
            if self.is_primary() {
                notify_critical(format!(
                    "CRITICAL {} {ticker}: close ({:?}) still unconfirmed after {}s (order {}, {} checks) — the broker may still hold the position; check the account",
                    self.spec.name,
                    p.reason,
                    age,
                    p.order_id.as_deref().unwrap_or("none"),
                    p.checks
                ));
            }
        }
        self.exit_pending.insert(ticker.to_string(), p);
    }

    /// re-check every pending exit (heartbeat).
    pub async fn check_exits_pending(&mut self, now: DateTime<Utc>, last_prices: &HashMap<String, f64>) {
        for ticker in self.exit_pending_tickers() {
            let last = last_prices.get(&ticker).copied();
            self.check_exit_pending(&ticker, now, last).await;
        }
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
        if self.unreconciled.contains_key(ticker) {
            self.reconcile_ticker(ticker, bar_ts).await;
        }
        if self.exit_pending.contains_key(ticker) {
            self.check_exit_pending(ticker, bar_ts, Some(last_price)).await;
        }

        // cross-ticker capacity: still tick (exits, clocks, diagnostics), but block entries
        let at_capacity = self.open_position_count() >= self.max_concurrent();
        // a ticker whose failed entry is still unreconciled against the broker must not
        // re-enter after the 5-minute suppression: the broker could already hold the first
        // order's shares and the engine would end up at half the real size
        let unreconciled = self.unreconciled.contains_key(ticker);
        let (mut result, mut trade, session_cfg_id) = {
            let session = self.sessions.get_mut(ticker)?;
            if (at_capacity && !session.has_position()) || unreconciled {
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
                            if (fill.quantity - shares).abs() > 0.5 {
                                warn!(book = %self.spec.name, ticker = %ticker, requested = shares, filled = fill.quantity, "broker filled a different quantity than the engine sized");
                            }
                            if let Some(s) = self.sessions.get_mut(ticker) {
                                if fill.quantity > 0.5 && fill.quantity < shares - 0.5 && s.resize_open(fill.quantity.floor()) {
                                    warn!(book = %self.spec.name, ticker = %ticker, engine_size = fill.quantity.floor(), "engine position resized to the partial fill");
                                }
                                if fill.fill_price > 0.0 {
                                    s.set_broker_entry_price(fill.fill_price);
                                }
                            }
                        }
                        Err(e) => self.handle_failed_entry(ticker, bar_ts, &e).await,
                    }
                }
            }
        }

        let mut dropped = Vec::new();
        if let Some(tws) = trade.take() {
            if self.exit_pending.contains_key(ticker) {
                // the exit stack re-fired while the earlier close is still unconfirmed:
                // nothing new to send; the engine stays in the position
                debug!(book = %self.spec.name, ticker, reason = ?tws.trade.exit_reason, "exit re-fired while a close is pending — reverted");
                self.revert_engine_close(ticker, &tws);
                result.event = TickEvent::Nothing;
            } else {
                match self.close_at_broker(ticker).await {
                    CloseOutcome::Filled(fill) => {
                        info!(book = %self.spec.name, ticker, fill_price = fill.fill_price, engine_price = tws.trade.exit_price, "broker position closed");
                        let broker_exit = Some(fill.fill_price).filter(|p| *p > 0.0);
                        self.daily_pnl += tws.trade.pnl;
                        self.record_trade(&tws, broker_exit).await;
                        trade = Some(tws);
                    }
                    CloseOutcome::Flat => {
                        warn!(book = %self.spec.name, ticker, "broker held no position at the close — recording the engine's exit");
                        self.daily_pnl += tws.trade.pnl;
                        self.record_trade(&tws, None).await;
                        trade = Some(tws);
                    }
                    CloseOutcome::Pending(order_id, why) => {
                        // the exit is NOT a trade until the broker says so
                        if self.revert_engine_close(ticker, &tws) {
                            self.begin_exit_pending(ticker, order_id, tws.trade.exit_reason.clone(), tws.trade.exit_price, bar_ts, &why);
                            result.event = TickEvent::Nothing;
                        } else {
                            error!(book = %self.spec.name, ticker, why, "broker close not confirmed and the engine could not reopen — recording the exit; CHECK THE ACCOUNT");
                            if self.is_primary() {
                                notify_critical(format!("CRITICAL {} {ticker}: broker close not confirmed ({why}) and the engine could not keep the position — exit recorded; check the account", self.spec.name));
                            }
                            self.daily_pnl += tws.trade.pnl;
                            self.record_trade(&tws, None).await;
                            trade = Some(tws);
                        }
                    }
                }
            }
            // this ticker is flat now — if a config is waiting, swap it in
            if trade.is_some() && self.pending.is_some() {
                dropped = self.apply_pending_where_flat().await;
            }
        }

        // observability: gate/near-miss events + state row
        self.record_block_events(ticker, bar_ts, &result, last_price, session_cfg_id).await;
        self.persist_state(ticker, shared).await;

        Some(BarOutcome { result, trade, dropped })
    }

    /// put the position the engine just closed back (the broker has not confirmed).
    fn revert_engine_close(&mut self, ticker: &str, tws: &TradeWithScores) -> bool {
        self.sessions
            .get_mut(ticker)
            .map(|s| s.reopen_after_unconfirmed_close(tws))
            .unwrap_or(false)
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
                Err(e) => {
                    error!(book = %self.spec.name, error = %e, "failed to write trade to database — spooling to logs/trade_spool.jsonl");
                    spool_trade(&self.spec.name, tws, broker_exit_price, &e.to_string());
                }
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
    /// an unconfirmed close keeps the position and goes
    /// pending like any other exit; a ticker already pending is only re-checked.
    async fn close_and_record(
        &mut self,
        ticker: &str,
        price: f64,
        reason: ExitReason,
        now: DateTime<Utc>,
    ) {
        if self.exit_pending.contains_key(ticker) {
            info!(book = %self.spec.name, ticker, ?reason, "force close requested while a close is pending — re-checking the pending order instead");
            self.check_exit_pending(ticker, now, Some(price).filter(|p| *p > 0.0)).await;
            return;
        }
        let broker_exit = match self.close_at_broker(ticker).await {
            CloseOutcome::Filled(fill) => {
                info!(book = %self.spec.name, ticker, fill_price = fill.fill_price, ?reason, "broker position closed");
                Some(fill.fill_price).filter(|p| *p > 0.0)
            }
            CloseOutcome::Flat => {
                warn!(book = %self.spec.name, ticker, ?reason, "broker held no position at the forced close — recording the engine's exit");
                None
            }
            CloseOutcome::Pending(order_id, why) => {
                self.begin_exit_pending(ticker, order_id, reason, price, now, &why);
                return;
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
            Some(mut engine) => {
                info!(book = %self.spec.name, ticker, version_id = pending_id, "engine rebuilt for new config");
                if let Some(old) = self.sessions.get(ticker) {
                    engine.carry_day_state(old.engine());
                }
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


/// last-resort record of a completed trade the database refused: one JSON line per trade
/// in `$LOG_DIR/trade_spool.jsonl` (default `logs/`), for a human to insert by hand. the
/// row is never lost silently; the file is append-only and tiny.
fn spool_trade(book: &str, tws: &TradeWithScores, broker_exit_price: Option<f64>, error: &str) {
    use std::io::Write;
    let dir = std::env::var("LOG_DIR").unwrap_or_else(|_| "logs".to_string());
    let path = std::path::Path::new(&dir).join("trade_spool.jsonl");
    let t = &tws.trade;
    let line = serde_json::json!({
        "spooled_at": Utc::now().to_rfc3339(),
        "book": book,
        "ticker": t.ticker,
        "direction": format!("{:?}", t.direction),
        "entry_price": t.entry_price,
        "exit_price": t.exit_price,
        "size": t.size,
        "entry_time": t.entry_time.to_rfc3339(),
        "exit_time": t.exit_time.to_rfc3339(),
        "pnl": t.pnl,
        "pnl_pct": t.pnl_pct,
        "exit_reason": format!("{:?}", t.exit_reason),
        "entry_reason": tws.entry_reason,
        "config_version_id": tws.config_version_id,
        "broker_entry_price": tws.broker_entry_price,
        "broker_exit_price": broker_exit_price,
        "db_error": error,
    });
    let res = std::fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| writeln!(f, "{line}"));
    if let Err(e) = res {
        error!(book, ticker = %t.ticker, error = %e, "could not write the trade spool either");
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

    // ── broker ↔ engine invariant ──

    use crate::broker::OrderFill;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use types::action::TradeDirection;

    /// a broker whose entry orders fail; what it reports holding is scripted per call.
    struct FailingBroker {
        /// answers to `open_positions`, consumed in order (the last repeats).
        positions: Mutex<Vec<Result<Vec<BrokerPosition>, String>>>,
        position_calls: AtomicUsize,
        closed: Mutex<Vec<String>>,
    }

    impl FailingBroker {
        fn new(positions: Vec<Result<Vec<BrokerPosition>, String>>) -> Self {
            Self { positions: Mutex::new(positions), position_calls: AtomicUsize::new(0), closed: Mutex::new(vec![]) }
        }
    }

    #[async_trait::async_trait]
    impl Broker for FailingBroker {
        fn kind(&self) -> &'static str {
            "alpaca_paper"
        }
        async fn submit_order(&self, _t: &str, _d: TradeDirection, _q: f64) -> Result<OrderFill, BrokerError> {
            Err(BrokerError::ConnectionError("order status poll failed: 422 invalid order_id".into()))
        }
        async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError> {
            self.closed.lock().unwrap().push(ticker.to_string());
            Ok(OrderFill { ticker: ticker.into(), direction: TradeDirection::Long, quantity: 1.0, fill_price: 100.0, filled_at: Utc::now() })
        }
        async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError> {
            self.position_calls.fetch_add(1, Ordering::SeqCst);
            let mut v = self.positions.lock().unwrap();
            let r = if v.len() > 1 { v.remove(0) } else { v.first().cloned().unwrap_or(Ok(vec![])) };
            r.map_err(BrokerError::ConnectionError)
        }
    }

    fn short(ticker: &str, qty: f64, px: f64) -> BrokerPosition {
        BrokerPosition { ticker: ticker.into(), direction: TradeDirection::Short, quantity: qty, entry_price: px, current_price: Some(px) }
    }

    fn primary_on(broker: Box<dyn Broker>, tickers: &[&str]) -> Book {
        let plan = BookPlan::new(spec("primary", BookRole::Primary, None), 12, test_config(tickers));
        Book::new_primary(plan, broker, 10_000.0, replay_sink(), Utc::now()).unwrap()
    }

    /// emulate the engine having just opened (PositionOpened) before the broker call.
    fn engine_opens(book: &mut Book, ticker: &str, ts: DateTime<Utc>) {
        book.session_mut(ticker)
            .unwrap()
            .engine_mut()
            .force_open_position(ticker.to_string(), TradeDirection::Short, 249.765, 12.0, ts)
            .unwrap();
    }

    #[tokio::test]
    async fn failed_order_with_a_broker_position_is_adopted_and_cooled_down() {
        // today's case: the poll failed but alpaca had filled 12 AMZN short @ 249.72
        let broker = FailingBroker::new(vec![Ok(vec![short("AMZN", 12.0, 249.72)])]);
        let mut book = primary_on(Box::new(broker), &["AMZN", "NVDA"]);
        let ts = Utc::now();
        engine_opens(&mut book, "AMZN", ts);
        let err = BrokerError::ConnectionError("422".into());
        book.handle_failed_entry("AMZN", ts, &err).await;

        let s = book.session("AMZN").unwrap();
        let p = s.current_position().expect("the broker's position is adopted, not forgotten");
        assert_eq!(p.direction, TradeDirection::Short);
        assert!((p.size - 12.0).abs() < f64::EPSILON);
        assert!((p.entry_price - 249.72).abs() < 1e-9, "adopted at the broker's fill, not the engine's price");
        assert!(s.entry_reason().unwrap().starts_with("adopted:failed-order"));
        assert_eq!(s.engine().entries_suppressed_until(), Some(ts + Duration::milliseconds(ORDER_FAIL_COOLDOWN_MS)));
        assert!(book.unreconciled_tickers().is_empty());
        assert_eq!(book.open_tickers(), vec!["AMZN".to_string()]);
    }

    #[tokio::test]
    async fn failed_order_with_a_flat_broker_stays_flat_and_gates_entries() {
        use types::test_fixtures::{make_market_state_ohlcv, ranging_ohlcv};
        let broker = FailingBroker::new(vec![Ok(vec![])]);
        let mut book = primary_on(Box::new(broker), &["AMZN"]);
        let ts = Utc::now();
        engine_opens(&mut book, "AMZN", ts);
        book.handle_failed_entry("AMZN", ts, &BrokerError::OrderRejected("cancelled".into())).await;

        let s = book.session("AMZN").unwrap();
        assert!(!s.has_position());
        assert!(book.unreconciled_tickers().is_empty());
        // the window cannot re-fire on the next bars
        let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &ranging_ohlcv(5, 100.0, 0.5));
        ms.timestamp = ts + Duration::minutes(1);
        assert_eq!(s.engine().entry_block_reason(&ms), Some("order_failed_cooldown"));
        ms.timestamp = ts + Duration::milliseconds(ORDER_FAIL_COOLDOWN_MS - 1);
        assert_eq!(s.engine().entry_block_reason(&ms), Some("order_failed_cooldown"));
        ms.timestamp = ts + Duration::milliseconds(ORDER_FAIL_COOLDOWN_MS);
        assert_ne!(s.engine().entry_block_reason(&ms), Some("order_failed_cooldown"));
    }

    #[tokio::test]
    async fn config_cooldown_wins_when_longer() {
        let mut cfg = test_config(&["AMZN"]);
        cfg.session.entry_cooldown_ms = 20 * 60 * 1000;
        let plan = BookPlan::new(spec("primary", BookRole::Primary, None), 12, cfg);
        let mut book = Book::new_primary(plan, Box::new(FailingBroker::new(vec![Ok(vec![])])), 10_000.0, replay_sink(), Utc::now()).unwrap();
        let ts = Utc::now();
        engine_opens(&mut book, "AMZN", ts);
        book.handle_failed_entry("AMZN", ts, &BrokerError::OrderRejected("x".into())).await;
        assert_eq!(book.session("AMZN").unwrap().engine().entries_suppressed_until(), Some(ts + Duration::minutes(20)));
    }

    #[tokio::test]
    async fn unreachable_broker_is_retried_on_the_next_bar_and_then_adopted() {
        use types::test_fixtures::{make_market_state_ohlcv, ranging_ohlcv};
        use chrono::TimeZone;
        let broker = FailingBroker::new(vec![Err("connection reset".into()), Ok(vec![short("AMZN", 12.0, 249.72)])]);
        let mut book = primary_on(Box::new(broker), &["AMZN"]);
        // inside regular hours, or the tick would (rightly) session-close the adopted position
        let ts = Utc.with_ymd_and_hms(2026, 10, 15, 14, 0, 0).unwrap();
        engine_opens(&mut book, "AMZN", ts);
        book.handle_failed_entry("AMZN", ts, &BrokerError::Unconfirmed("state unknown".into())).await;
        assert!(!book.session("AMZN").unwrap().has_position());
        assert_eq!(book.unreconciled_tickers(), vec!["AMZN".to_string()]);

        // next bar: the broker answers and the position is adopted before the tick
        let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &ranging_ohlcv(5, 100.0, 0.5));
        ms.timestamp = ts + Duration::minutes(1);
        let shared = TickerShared { last_bar_at: Some(ms.timestamp), last_price: Some(ms.last_price), feed_stale: false };
        let out = book.on_bar("AMZN", ms, &shared).await.unwrap();
        assert!(out.trade.is_none(), "{:?}", out.trade.map(|t| t.trade.exit_reason));
        assert!(book.unreconciled_tickers().is_empty());
        let p = book.session("AMZN").unwrap().current_position().expect("adopted on the retry");
        assert!((p.size - 12.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn a_shadow_never_reconciles_against_a_broker() {
        let plan = BookPlan::new(spec("shadow:x", BookRole::Shadow, None), 10, test_config(&["AMZN"]));
        let mut book = Book::new_shadow(plan, 10_000.0, replay_sink(), 0.0, Utc::now()).unwrap();
        let ts = Utc::now();
        engine_opens(&mut book, "AMZN", ts);
        book.handle_failed_entry("AMZN", ts, &BrokerError::OrderRejected("x".into())).await;
        assert!(!book.session("AMZN").unwrap().has_position());
        assert!(book.unreconciled_tickers().is_empty());
        assert!(book.session("AMZN").unwrap().engine().entries_suppressed_until().is_some());
    }

    async fn simulated_with_orphans() -> Box<dyn Broker> {
        let b = SimulatedBroker::new(0.0);
        b.set_last_price_for("SPY", 500.0);
        b.set_last_price_for("AMD", 150.0);
        b.submit_order("SPY", TradeDirection::Short, 48.0).await.unwrap();
        b.submit_order("AMD", TradeDirection::Long, 7.0).await.unwrap();
        Box::new(b)
    }

    #[tokio::test]
    async fn startup_adopt_policy_adopts_traded_tickers_and_closes_the_rest() {
        let mut book = primary_on(simulated_with_orphans().await, &["SPY", "QQQ"]);
        let out = book.reconcile_orphans(OrphanPolicy::Adopt, Utc::now()).await.unwrap();
        let mut by: Vec<(String, OrphanAction)> = out.into_iter().map(|o| (o.ticker, o.action)).collect();
        by.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(by, vec![("AMD".to_string(), OrphanAction::Closed), ("SPY".to_string(), OrphanAction::Adopted)]);
        let p = book.session("SPY").unwrap().current_position().expect("SPY adopted");
        assert_eq!(p.direction, TradeDirection::Short);
        assert!((p.size - 48.0).abs() < f64::EPSILON);
        assert!((p.entry_price - 500.0).abs() < f64::EPSILON);
        assert_eq!(book.session("SPY").unwrap().entry_reason(), Some("adopted:orphan-at-start"));
        // the broker now holds only what the engine knows
        let held = book.broker_open_positions().await.unwrap();
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].ticker, "SPY");
        // a second pass finds nothing to do
        assert!(book.reconcile_orphans(OrphanPolicy::Adopt, Utc::now()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn startup_close_policy_closes_everything() {
        let mut book = primary_on(simulated_with_orphans().await, &["SPY"]);
        let out = book.reconcile_orphans(OrphanPolicy::Close, Utc::now()).await.unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|o| o.action == OrphanAction::Closed));
        assert!(!book.session("SPY").unwrap().has_position());
        assert!(book.broker_open_positions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn startup_ignore_policy_only_reports() {
        let mut book = primary_on(simulated_with_orphans().await, &["SPY"]);
        let out = book.reconcile_orphans(OrphanPolicy::Ignore, Utc::now()).await.unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|o| o.action == OrphanAction::Ignored));
        assert!(!book.session("SPY").unwrap().has_position());
        assert_eq!(book.broker_open_positions().await.unwrap().len(), 2);
    }

    #[test]
    fn orphan_policy_parses() {
        assert_eq!(OrphanPolicy::parse("adopt"), Some(OrphanPolicy::Adopt));
        assert_eq!(OrphanPolicy::parse(" Close "), Some(OrphanPolicy::Close));
        assert_eq!(OrphanPolicy::parse("IGNORE"), Some(OrphanPolicy::Ignore));
        assert_eq!(OrphanPolicy::parse("nope"), None);
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

    // ── exits the broker has not confirmed (2026-10-16) + intraday reconciliation ──

    use crate::broker::OrderView;
    use chrono::TimeZone;
    use types::test_fixtures::{make_market_state_ohlcv, ranging_ohlcv};

    /// one scripted answer to `close_position`.
    #[derive(Clone)]
    enum CloseAnswer {
        Fill(f64),
        Working(&'static str),
        NoPosition,
        Conn(&'static str),
    }

    /// a broker whose closes, order lookups and position lists are scripted (each list
    /// is consumed in order; the last answer repeats).
    struct ScriptedBroker {
        closes: Mutex<Vec<CloseAnswer>>,
        orders: Mutex<Vec<Result<OrderView, String>>>,
        positions: Mutex<Vec<Result<Vec<BrokerPosition>, String>>>,
        /// every call, shared with the test (the broker is boxed into the book).
        calls: Calls,
    }

    type Calls = Arc<Mutex<Vec<String>>>;

    impl ScriptedBroker {
        fn boxed(closes: Vec<CloseAnswer>, orders: Vec<Result<OrderView, String>>, positions: Vec<Result<Vec<BrokerPosition>, String>>) -> (Box<dyn Broker>, Calls) {
            let calls: Calls = Arc::new(Mutex::new(vec![]));
            let b = Self { closes: Mutex::new(closes), orders: Mutex::new(orders), positions: Mutex::new(positions), calls: calls.clone() };
            (Box::new(b), calls)
        }
        fn next<T: Clone>(v: &mut Vec<T>) -> Option<T> {
            if v.len() > 1 { Some(v.remove(0)) } else { v.first().cloned() }
        }
    }

    fn calls_of(c: &Calls) -> Vec<String> {
        c.lock().unwrap().clone()
    }

    #[async_trait::async_trait]
    impl Broker for ScriptedBroker {
        fn kind(&self) -> &'static str {
            "alpaca_paper"
        }
        async fn submit_order(&self, t: &str, d: TradeDirection, q: f64) -> Result<OrderFill, BrokerError> {
            self.calls.lock().unwrap().push(format!("submit:{t}"));
            Ok(OrderFill { ticker: t.into(), direction: d, quantity: q, fill_price: 100.0, filled_at: Utc::now() })
        }
        async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError> {
            self.calls.lock().unwrap().push(format!("close:{ticker}"));
            match Self::next(&mut self.closes.lock().unwrap()).unwrap_or(CloseAnswer::Conn("unscripted")) {
                CloseAnswer::Fill(px) => Ok(OrderFill { ticker: ticker.into(), direction: TradeDirection::Long, quantity: 48.0, fill_price: px, filled_at: Utc::now() }),
                CloseAnswer::Working(id) => Err(BrokerError::Working { order_id: id.to_string() }),
                CloseAnswer::NoPosition => Err(BrokerError::NoPosition(ticker.into())),
                CloseAnswer::Conn(e) => Err(BrokerError::ConnectionError(e.into())),
            }
        }
        async fn order_status(&self, order_id: &str) -> Result<OrderView, BrokerError> {
            self.calls.lock().unwrap().push(format!("order:{order_id}"));
            Self::next(&mut self.orders.lock().unwrap())
                .unwrap_or(Err("unscripted".into()))
                .map_err(BrokerError::ConnectionError)
        }
        async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError> {
            self.calls.lock().unwrap().push("positions".into());
            Self::next(&mut self.positions.lock().unwrap())
                .unwrap_or(Ok(vec![]))
                .map_err(BrokerError::ConnectionError)
        }
    }

    fn order(id: &str, state: OrderState, filled_qty: f64, px: Option<f64>) -> OrderView {
        OrderView { id: id.into(), client_order_id: "cid".into(), symbol: "AMZN".into(), state, filled_qty, avg_fill_price: px, filled_at: None }
    }

    /// a primary whose exit stack closes any position held longer than a minute.
    fn primary_with_timeout(broker: Box<dyn Broker>, tickers: &[&str], log: ReplayLog) -> Book {
        let mut cfg = test_config(tickers);
        cfg.actions.push(types::ActionConfig {
            action_type: "max_hold_timeout".to_string(),
            instance_id: "mh".to_string(),
            phase: types::ActionPhase::Exit,
            enabled: true,
            priority: 0,
            params: vec![("max_hold_ms", json!(60_000))].into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            last_modified_by: None,
            last_modified_at: None,
            modification_reason: None,
        });
        let plan = BookPlan::new(spec("primary", BookRole::Primary, None), 12, cfg);
        Book::new_primary(plan, broker, 10_000.0, Sink::Replay(log), Utc::now()).unwrap()
    }

    fn rth(h: u32, m: u32) -> DateTime<Utc> {
        // 2026-10-15 is a thursday; 14:00Z = 10:00 ET
        Utc.with_ymd_and_hms(2026, 10, 15, h, m, 0).unwrap()
    }

    fn bar(ts: DateTime<Utc>, price: f64) -> (MarketState, TickerShared) {
        let mut ms = make_market_state_ohlcv(Timescale::FiveMinute, &ranging_ohlcv(5, 100.0, 0.5));
        ms.timestamp = ts;
        ms.last_price = price;
        let shared = TickerShared { last_bar_at: Some(ts), last_price: Some(price), feed_stale: false };
        (ms, shared)
    }

    fn recorded(log: &ReplayLog) -> Vec<ReplayTrade> {
        log.lock().unwrap().clone()
    }

    /// today's incident, fixed: the close comes back `Working`; the engine keeps the
    /// position and no trade is recorded; the order fills before a later bar and the trade
    /// is recorded with the broker's fill price and the original exit reason.
    #[tokio::test]
    async fn unconfirmed_close_keeps_the_position_until_the_order_fills() {
        let log: ReplayLog = Arc::new(Mutex::new(Vec::new()));
        let (broker, calls) = ScriptedBroker::boxed(
            vec![CloseAnswer::Working("ord-1")],
            vec![Ok(order("ord-1", OrderState::Working, 0.0, None)), Ok(order("ord-1", OrderState::Filled, 48.0, Some(250.10)))],
            vec![Ok(vec![short("AMZN", 48.0, 249.70)])],
        );
        let mut book = primary_with_timeout(broker, &["AMZN"], log.clone());
        let t0 = rth(14, 0);
        book.session_mut("AMZN").unwrap().adopt_broker_position("AMZN", TradeDirection::Short, 48.0, 249.70, t0, None, "adopted:orphan-at-start").unwrap();

        // bar 1: the timeout fires → close → Working: NOT a trade
        let (ms, shared) = bar(t0 + Duration::minutes(2), 254.82);
        let out = book.on_bar("AMZN", ms, &shared).await.unwrap();
        assert!(out.trade.is_none(), "an unconfirmed close is not a trade");
        assert!(matches!(out.result.event, TickEvent::Nothing));
        assert!(book.session("AMZN").unwrap().has_position(), "engine keeps the position");
        assert_eq!(book.session("AMZN").unwrap().entry_reason(), Some("adopted:orphan-at-start"));
        assert_eq!(book.exit_pending_tickers(), vec!["AMZN".to_string()]);
        let p = book.exit_pending("AMZN").unwrap();
        assert_eq!(p.order_id.as_deref(), Some("ord-1"));
        assert_eq!(p.reason, ExitReason::MaxHoldTimeout);
        assert!(recorded(&log).is_empty());
        assert!(book.daily_pnl().abs() < f64::EPSILON);
        assert_eq!(book.open_tickers(), vec!["AMZN".to_string()]);

        // bar 2: still working; the exit stack re-fires and is reverted, nothing new is sent
        let (ms, shared) = bar(t0 + Duration::minutes(3), 254.90);
        let out = book.on_bar("AMZN", ms, &shared).await.unwrap();
        assert!(out.trade.is_none());
        assert!(book.session("AMZN").unwrap().has_position());
        assert_eq!(book.exit_pending("AMZN").unwrap().checks, 1);
        assert!(recorded(&log).is_empty());

        // bar 3: filled @ 250.10 → recorded with the real fill, original reason, engine flat
        let (ms, shared) = bar(t0 + Duration::minutes(4), 255.00);
        let _ = book.on_bar("AMZN", ms, &shared).await.unwrap();
        assert!(!book.session("AMZN").unwrap().has_position());
        assert!(book.exit_pending_tickers().is_empty());
        let trades = recorded(&log);
        assert_eq!(trades.len(), 1);
        let t = &trades[0];
        assert_eq!(t.tws.trade.exit_reason, ExitReason::MaxHoldTimeout);
        assert_eq!(t.broker_exit_price, Some(250.10));
        assert!((t.tws.trade.exit_price - 250.10).abs() < 1e-9);
        assert!((t.tws.trade.entry_price - 249.70).abs() < 1e-9);
        assert_eq!(t.tws.entry_reason, "adopted:orphan-at-start");
        assert_eq!(t.tws.broker_entry_price, Some(249.70));
        let expect_pnl = (249.70 - 250.10) * 48.0;
        assert!((t.tws.trade.pnl - expect_pnl).abs() < 1e-6);
        assert!((book.daily_pnl() - expect_pnl).abs() < 1e-6);
        // exactly one close went to the broker
        let calls = calls_of(&calls);
        assert_eq!(calls.iter().filter(|c| c.starts_with("close:")).count(), 1, "{calls:?}");
    }

    /// the pending order ends (the human cancelled it) and the broker is flat (the human
    /// closed it): the exit is recorded at the last known price and the ticker clears.
    #[tokio::test]
    async fn pending_exit_resolves_when_the_broker_turns_out_flat() {
        let log: ReplayLog = Arc::new(Mutex::new(Vec::new()));
        let (broker, calls) = ScriptedBroker::boxed(
            vec![CloseAnswer::Working("ord-1")],
            vec![Ok(order("ord-1", OrderState::Terminal("Canceled".into()), 0.0, None))],
            vec![Ok(vec![])],
        );
        let mut book = primary_with_timeout(broker, &["AMZN"], log.clone());
        let t0 = rth(14, 0);
        book.session_mut("AMZN").unwrap().adopt_broker_position("AMZN", TradeDirection::Short, 48.0, 249.70, t0, None, "x").unwrap();
        let (ms, shared) = bar(t0 + Duration::minutes(2), 254.82);
        book.on_bar("AMZN", ms, &shared).await.unwrap();
        assert!(book.exit_pending("AMZN").is_some());

        // heartbeat path, last price 251.00
        let mut prices = HashMap::new();
        prices.insert("AMZN".to_string(), 251.0);
        book.check_exits_pending(t0 + Duration::minutes(3), &prices).await;
        assert!(book.exit_pending_tickers().is_empty());
        assert!(!book.session("AMZN").unwrap().has_position());
        let trades = recorded(&log);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].tws.trade.exit_reason, ExitReason::MaxHoldTimeout);
        assert!((trades[0].tws.trade.exit_price - 251.0).abs() < 1e-9);
        assert_eq!(trades[0].broker_exit_price, None, "fill unknown");
        let calls = calls_of(&calls);
        assert_eq!(calls.iter().filter(|c| c.starts_with("close:")).count(), 1, "{calls:?}");
    }

    /// the force-exit safety net uses the same path: an unconfirmed force close keeps the
    /// position; on the next heartbeat the broker still holds it after the order ended,
    /// so a new close is submitted and its fill is recorded.
    #[tokio::test]
    async fn unconfirmed_force_close_keeps_the_position_and_resubmits() {
        let log: ReplayLog = Arc::new(Mutex::new(Vec::new()));
        let (broker, calls) = ScriptedBroker::boxed(
            vec![CloseAnswer::Conn("connection reset"), CloseAnswer::Fill(250.00)],
            vec![],
            vec![Ok(vec![short("AMZN", 48.0, 249.70)])],
        );
        let mut book = primary_on(broker, &["AMZN"]);
        // swap the sink for one we can read
        book.sink = Sink::Replay(log.clone());
        let t0 = rth(14, 0);
        book.session_mut("AMZN").unwrap().adopt_broker_position("AMZN", TradeDirection::Short, 48.0, 249.70, t0, None, "x").unwrap();
        let mut prices = HashMap::new();
        prices.insert("AMZN".to_string(), 252.0);

        book.flatten_all(ExitReason::SessionClose, &prices, t0 + Duration::minutes(5)).await;
        assert!(book.session("AMZN").unwrap().has_position(), "no pretending: the position stays until the broker confirms");
        assert!(recorded(&log).is_empty());
        let p = book.exit_pending("AMZN").unwrap();
        assert_eq!(p.order_id, None);
        assert_eq!(p.reason, ExitReason::SessionClose);

        // a second flatten (the clock safety net again) does not send another close
        book.flatten_all(ExitReason::SessionClose, &prices, t0 + Duration::minutes(6)).await;
        // …but that re-check saw the broker still holding → re-submitted → filled
        assert!(book.exit_pending_tickers().is_empty());
        assert!(!book.session("AMZN").unwrap().has_position());
        let trades = recorded(&log);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].tws.trade.exit_reason, ExitReason::SessionClose);
        assert_eq!(trades[0].broker_exit_price, Some(250.0));
        assert!((trades[0].tws.trade.exit_price - 250.0).abs() < 1e-9);
        let calls = calls_of(&calls);
        assert_eq!(calls.iter().filter(|c| c.starts_with("close:")).count(), 2, "{calls:?}");
    }

    /// the broker already holds nothing when the exit fires (someone closed it by hand):
    /// the engine's exit is recorded as before, with no broker fill price.
    #[tokio::test]
    async fn close_with_no_broker_position_records_the_engine_exit() {
        let log: ReplayLog = Arc::new(Mutex::new(Vec::new()));
        let (broker, calls) = ScriptedBroker::boxed(vec![CloseAnswer::NoPosition], vec![], vec![Ok(vec![])]);
        let mut book = primary_with_timeout(broker, &["AMZN"], log.clone());
        let t0 = rth(14, 0);
        book.session_mut("AMZN").unwrap().adopt_broker_position("AMZN", TradeDirection::Short, 48.0, 249.70, t0, None, "x").unwrap();
        let (ms, shared) = bar(t0 + Duration::minutes(2), 254.82);
        let out = book.on_bar("AMZN", ms, &shared).await.unwrap();
        let tws = out.trade.expect("recorded");
        assert_eq!(tws.trade.exit_reason, ExitReason::MaxHoldTimeout);
        assert!(!book.session("AMZN").unwrap().has_position());
        assert!(book.exit_pending_tickers().is_empty());
        let trades = recorded(&log);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].broker_exit_price, None);
        assert_eq!(calls_of(&calls).iter().filter(|c| c.starts_with("close:")).count(), 1);
    }

    #[tokio::test]
    async fn pending_exit_alerts_once_after_the_deadline_and_stays_pending() {
        let (broker, _calls) = ScriptedBroker::boxed(
            vec![CloseAnswer::Working("ord-1")],
            vec![Ok(order("ord-1", OrderState::Working, 20.0, Some(249.9)))],
            vec![Ok(vec![short("AMZN", 28.0, 249.70)])],
        );
        let mut book = primary_on(broker, &["AMZN"]);
        let t0 = rth(14, 0);
        book.session_mut("AMZN").unwrap().adopt_broker_position("AMZN", TradeDirection::Short, 48.0, 249.70, t0, None, "x").unwrap();
        let prices: HashMap<String, f64> = [("AMZN".to_string(), 252.0)].into_iter().collect();
        book.flatten_all(ExitReason::SessionClose, &prices, t0).await;
        assert!(!book.exit_pending("AMZN").unwrap().alerted);
        book.check_exits_pending(t0 + Duration::seconds(EXIT_PENDING_ALERT_SECS - 1), &prices).await;
        assert!(!book.exit_pending("AMZN").unwrap().alerted);
        book.check_exits_pending(t0 + Duration::seconds(EXIT_PENDING_ALERT_SECS), &prices).await;
        let p = book.exit_pending("AMZN").unwrap();
        assert!(p.alerted);
        assert_eq!(p.checks, 2);
        assert!(book.session("AMZN").unwrap().has_position(), "a partial fill keeps waiting");
    }

    #[tokio::test]
    async fn heartbeat_reconciliation_adopts_an_orphan_after_two_sightings() {
        let (broker, calls) = ScriptedBroker::boxed(
            vec![CloseAnswer::Fill(150.0)],
            vec![],
            vec![Ok(vec![short("AMZN", 48.0, 249.70), BrokerPosition { ticker: "AMD".into(), direction: TradeDirection::Long, quantity: 7.0, entry_price: 150.0, current_price: None }])],
        );
        let mut book = primary_on(broker, &["AMZN", "NVDA"]);
        let prices = HashMap::new();
        let t0 = rth(14, 0);

        // first sighting: nothing happens
        let (orphans, flat) = book.reconcile_live(OrphanPolicy::Adopt, t0, &prices).await.unwrap();
        assert!(orphans.is_empty() && flat.is_empty());
        assert!(!book.session("AMZN").unwrap().has_position());
        assert!(!calls_of(&calls).iter().any(|c| c.starts_with("close:")));

        // second consecutive sighting: AMZN (traded) adopted, AMD (not traded) closed
        let (orphans, flat) = book.reconcile_live(OrphanPolicy::Adopt, t0 + Duration::minutes(5), &prices).await.unwrap();
        assert!(flat.is_empty());
        let mut by: Vec<(String, OrphanAction)> = orphans.into_iter().map(|o| (o.ticker, o.action)).collect();
        by.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(by, vec![("AMD".to_string(), OrphanAction::Closed), ("AMZN".to_string(), OrphanAction::Adopted)]);
        let p = book.session("AMZN").unwrap().current_position().expect("adopted");
        assert_eq!(p.direction, TradeDirection::Short);
        assert!((p.size - 48.0).abs() < f64::EPSILON);
        assert_eq!(book.session("AMZN").unwrap().entry_reason(), Some("adopted:orphan-at-intraday"));
        assert_eq!(calls_of(&calls).iter().filter(|c| c == &"close:AMD").count(), 1);

        // a third check: the engine knows AMZN now; AMD's sighting count restarts
        let (orphans, _) = book.reconcile_live(OrphanPolicy::Adopt, t0 + Duration::minutes(10), &prices).await.unwrap();
        assert!(orphans.is_empty(), "AMZN is known; AMD needs two fresh sightings");
    }

    #[tokio::test]
    async fn heartbeat_reconciliation_debounce_resets_when_the_mismatch_clears() {
        let (broker, _calls) = ScriptedBroker::boxed(
            vec![],
            vec![],
            vec![Ok(vec![short("AMZN", 48.0, 249.70)]), Ok(vec![]), Ok(vec![short("AMZN", 48.0, 249.70)]), Ok(vec![])],
        );
        let mut book = primary_on(broker, &["AMZN"]);
        let prices = HashMap::new();
        for i in 0..4 {
            let (orphans, flat) = book.reconcile_live(OrphanPolicy::Adopt, rth(14, i), &prices).await.unwrap();
            assert!(orphans.is_empty() && flat.is_empty(), "check {i}: an in-flight order must not be adopted");
        }
        assert!(!book.session("AMZN").unwrap().has_position());
    }

    #[tokio::test]
    async fn heartbeat_reconciliation_marks_broker_flat_after_two_sightings() {
        let log: ReplayLog = Arc::new(Mutex::new(Vec::new()));
        let (broker, calls) = ScriptedBroker::boxed(vec![], vec![], vec![Ok(vec![])]);
        let mut book = primary_on(broker, &["AMZN"]);
        book.sink = Sink::Replay(log.clone());
        let t0 = rth(14, 0);
        book.session_mut("AMZN").unwrap().adopt_broker_position("AMZN", TradeDirection::Short, 48.0, 249.70, t0, None, "x").unwrap();
        let prices: HashMap<String, f64> = [("AMZN".to_string(), 251.0)].into_iter().collect();

        let (_, flat) = book.reconcile_live(OrphanPolicy::Adopt, t0 + Duration::minutes(5), &prices).await.unwrap();
        assert!(flat.is_empty());
        assert!(book.session("AMZN").unwrap().has_position());

        let (_, flat) = book.reconcile_live(OrphanPolicy::Adopt, t0 + Duration::minutes(10), &prices).await.unwrap();
        assert_eq!(flat, vec!["AMZN".to_string()]);
        assert!(!book.session("AMZN").unwrap().has_position());
        let trades = recorded(&log);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].tws.trade.exit_reason, ExitReason::ManualOverride);
        assert!((trades[0].tws.trade.exit_price - 251.0).abs() < 1e-9);
        assert_eq!(trades[0].broker_exit_price, None);
        assert!((book.daily_pnl() - (249.70 - 251.0) * 48.0).abs() < 1e-6);
        // nothing was sent to the broker: it already holds nothing
        assert!(!calls_of(&calls).iter().any(|c| c.starts_with("close:")));
    }

    /// a ticker whose exit is pending is the broker's to answer, not the reconciler's.
    #[tokio::test]
    async fn heartbeat_reconciliation_skips_tickers_with_an_exit_pending() {
        let (broker, _calls) = ScriptedBroker::boxed(vec![CloseAnswer::Working("ord-1")], vec![Ok(order("ord-1", OrderState::Working, 0.0, None))], vec![Ok(vec![])]);
        let mut book = primary_on(broker, &["AMZN"]);
        let t0 = rth(14, 0);
        book.session_mut("AMZN").unwrap().adopt_broker_position("AMZN", TradeDirection::Short, 48.0, 249.70, t0, None, "x").unwrap();
        let prices: HashMap<String, f64> = [("AMZN".to_string(), 251.0)].into_iter().collect();
        book.flatten_all(ExitReason::SessionClose, &prices, t0).await;
        assert!(book.exit_pending("AMZN").is_some());
        for i in 1..=3 {
            let (orphans, flat) = book.reconcile_live(OrphanPolicy::Adopt, t0 + Duration::minutes(5 * i), &prices).await.unwrap();
            assert!(orphans.is_empty() && flat.is_empty());
        }
        assert!(book.session("AMZN").unwrap().has_position());
        assert!(book.exit_pending("AMZN").is_some());
    }

}
