use std::fmt;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::{debug, error, info, warn};
use types::action::TradeDirection;

/// result of a filled order.
#[derive(Debug, Clone)]
pub struct OrderFill {
    pub ticker: String,
    pub direction: TradeDirection,
    pub quantity: f64,
    pub fill_price: f64,
    pub filled_at: DateTime<Utc>,
}

/// broker errors.
#[derive(Debug)]
pub enum BrokerError {
    OrderRejected(String),
    ConnectionError(String),
    NoPosition(String),
    /// the broker could not be asked whether the order filled — the caller must
    /// reconcile against the broker's positions before trusting either outcome.
    Unconfirmed(String),
    /// the order is still working at the deadline and was deliberately left so.
    OrderWorking(String),
}

impl fmt::Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BrokerError::OrderRejected(msg) => write!(f, "order rejected: {msg}"),
            BrokerError::ConnectionError(msg) => write!(f, "connection error: {msg}"),
            BrokerError::NoPosition(msg) => write!(f, "no position: {msg}"),
            BrokerError::Unconfirmed(msg) => write!(f, "order state UNCONFIRMED: {msg}"),
            BrokerError::OrderWorking(id) => write!(f, "order {id} still working"),
        }
    }
}

impl std::error::Error for BrokerError {}

/// a position the broker currently holds.
#[derive(Debug, Clone)]
pub struct BrokerPosition {
    pub ticker: String,
    pub direction: TradeDirection,
    pub quantity: f64,
    pub entry_price: f64,
    /// the broker's mark, when it reports one (alpaca `current_price`).
    pub current_price: Option<f64>,
}

/// outcome of closing a position the engine never owned.
#[derive(Debug, Clone)]
pub enum OrphanClose {
    Filled(OrderFill),
    /// an after-hours day limit is resting at the broker; it fills or expires on its own.
    Working { order_id: String, limit_price: f64 },
}

/// trait for order execution. implementations can be live (alpaca) or simulated.
#[async_trait]
pub trait Broker: Send + Sync {
    async fn submit_order(
        &self,
        ticker: &str,
        direction: TradeDirection,
        quantity: f64,
    ) -> Result<OrderFill, BrokerError>;

    async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError>;

    /// positions the broker currently holds (for startup reconciliation).
    async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError>;

    /// close a position the engine does not own (startup orphan). with the market open a
    /// market order; otherwise an extended-hours day limit at the last trade ± 0.3 % that
    /// is left working. default (simulated brokers): `close_position`.
    async fn close_orphan(&self, pos: &BrokerPosition, market_open: bool) -> Result<OrphanClose, BrokerError> {
        let _ = market_open;
        self.close_position(&pos.ticker).await.map(OrphanClose::Filled)
    }

    /// update the market price. default no-op for brokers that don't need it.
    fn set_last_price(&self, _price: f64) {}

    /// update the market price of one ticker. brokers without per-ticker prices fall
    /// back to `set_last_price`; the simulated broker fills `ticker` from this price.
    fn set_last_price_for(&self, _ticker: &str, price: f64) {
        self.set_last_price(price);
    }

    /// which implementation this is ("simulated" | "alpaca_paper"); lets a book prove
    /// what it is wired to without exposing the broker.
    fn kind(&self) -> &'static str;
}

/// simulated broker that fills at the ticker's last price ± slippage. no external deps.
pub struct SimulatedBroker {
    /// slippage in basis points (e.g., 5 = 0.05% = 5 bps).
    slippage_bps: f64,
    /// most recent market price of any ticker — the fallback when a ticker has no
    /// price of its own (`set_last_price`, the pre-books interface).
    last_price: std::sync::atomic::AtomicU64,
    /// per-ticker last prices (`set_last_price_for`), so a flatten of several tickers
    /// fills each at its own price rather than the last-arrived ticker's.
    ticker_prices: std::sync::Mutex<std::collections::HashMap<String, f64>>,
    /// track simulated positions: ticker → (direction, quantity, entry_price).
    positions: tokio::sync::Mutex<std::collections::HashMap<String, (TradeDirection, f64, f64)>>,
}

impl SimulatedBroker {
    pub fn new(slippage_bps: f64) -> Self {
        Self {
            slippage_bps,
            last_price: std::sync::atomic::AtomicU64::new(0),
            ticker_prices: std::sync::Mutex::new(std::collections::HashMap::new()),
            positions: tokio::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn slippage_bps(&self) -> f64 {
        self.slippage_bps
    }

    fn get_last_price(&self) -> f64 {
        f64::from_bits(
            self.last_price
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    /// the ticker's own last price when one was set, else the global last price.
    fn price_for(&self, ticker: &str) -> f64 {
        let own = self
            .ticker_prices
            .lock()
            .ok()
            .and_then(|m| m.get(ticker).copied())
            .filter(|p| *p > 0.0);
        own.unwrap_or_else(|| self.get_last_price())
    }

    fn apply_slippage(&self, price: f64, direction: &TradeDirection) -> f64 {
        let slip = price * self.slippage_bps / 10_000.0;
        match direction {
            TradeDirection::Long => price + slip,  // buy at higher price
            TradeDirection::Short => price - slip, // sell at lower price
        }
    }
}

#[async_trait]
impl Broker for SimulatedBroker {
    fn set_last_price(&self, price: f64) {
        self.last_price
            .store(price.to_bits(), std::sync::atomic::Ordering::Relaxed);
    }

    fn set_last_price_for(&self, ticker: &str, price: f64) {
        if let Ok(mut m) = self.ticker_prices.lock() {
            m.insert(ticker.to_string(), price);
        }
        self.set_last_price(price);
    }

    fn kind(&self) -> &'static str {
        "simulated"
    }

    async fn submit_order(
        &self,
        ticker: &str,
        direction: TradeDirection,
        quantity: f64,
    ) -> Result<OrderFill, BrokerError> {
        let base_price = self.price_for(ticker);
        if base_price <= 0.0 {
            return Err(BrokerError::OrderRejected(
                "last_price not set".to_string(),
            ));
        }

        let fill_price = self.apply_slippage(base_price, &direction);

        // track position
        let mut positions = self.positions.lock().await;
        positions.insert(
            ticker.to_string(),
            (direction, quantity, fill_price),
        );

        Ok(OrderFill {
            ticker: ticker.to_string(),
            direction,
            quantity,
            fill_price,
            filled_at: Utc::now(),
        })
    }

    async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError> {
        let positions = self.positions.lock().await;
        Ok(positions
            .iter()
            .map(|(t, (d, q, p))| BrokerPosition {
                ticker: t.clone(),
                direction: *d,
                quantity: *q,
                entry_price: *p,
                current_price: Some(self.price_for(t)).filter(|x| *x > 0.0),
            })
            .collect())
    }

    async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError> {
        let mut positions = self.positions.lock().await;
        let (direction, quantity, _entry_price) = positions
            .remove(ticker)
            .ok_or_else(|| BrokerError::NoPosition(ticker.to_string()))?;

        let base_price = self.price_for(ticker);
        // closing: reverse direction for slippage
        let close_direction = match direction {
            TradeDirection::Long => TradeDirection::Short,
            TradeDirection::Short => TradeDirection::Long,
        };
        let fill_price = self.apply_slippage(base_price, &close_direction);

        Ok(OrderFill {
            ticker: ticker.to_string(),
            direction: close_direction,
            quantity,
            fill_price,
            filled_at: Utc::now(),
        })
    }
}


/// convert an apca Num to f64.
fn num_to_f64(n: &num_decimal::Num) -> f64 {
    n.to_f64().unwrap_or(0.0)
}

/// a price as an apca Num with two decimals (alpaca's limit-price precision for >= $1).
fn num_from_price(price: f64) -> Result<num_decimal::Num, BrokerError> {
    use std::str::FromStr;
    num_decimal::Num::from_str(&format!("{price:.2}"))
        .map_err(|e| BrokerError::OrderRejected(format!("bad price {price}: {e:?}")))
}

/// the `client_order_id` every order we submit carries: `gtf-<book>-<ticker>-<unix_ms>`.
/// alpaca caps it at 128 characters; anything that is not `[A-Za-z0-9_.-]` becomes `-`.
pub fn client_order_id(book: &str, ticker: &str, unix_ms: i64) -> String {
    let clean = |s: &str| -> String {
        s.chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' { c } else { '-' })
            .collect()
    };
    let mut id = format!("gtf-{}-{}-{}", clean(book), clean(ticker), unix_ms);
    id.truncate(128);
    id
}

// ── order lookups that do NOT put the id in the path the way apca 0.30 does ──
//
// apca 0.30 formats `/v2/orders/{id}` with `Uuid::as_simple()` (no hyphens); since
// 2026-10-15 alpaca answers that with 422 `invalid order_id`. these endpoints send the
// id string exactly as alpaca returned it (hyphenated) and otherwise reuse apca's
// client (base url, auth headers) and types.

/// the raw id string alpaca returned, hyphenated — what the order endpoints must send.
pub fn order_id_string(id: &apca::api::v2::order::Id) -> String {
    id.as_hyphenated().to_string()
}

http_endpoint::EndpointDef! {
    /// GET /v2/orders/{id} with the id sent verbatim (hyphenated).
    pub OrderGetRaw(String),
    Ok => apca::api::v2::order::Order, [
        /// the order was retrieved.
        OK,
    ],
    Err => OrderGetRawError, [
        /// no order with that id.
        NOT_FOUND => NotFound,
        /// alpaca did not accept the id.
        UNPROCESSABLE_ENTITY => InvalidInput,
    ],
    ConversionErr => serde_json::Error,
    ApiErr => apca::ApiError,

    fn path(input: &Self::Input) -> std::borrow::Cow<'static, str> {
        format!("/v2/orders/{input}").into()
    }

    fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
        serde_json::from_slice::<Self::Output>(body)
    }

    fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
        serde_json::from_slice::<Self::ApiError>(body).map_err(|_| body.to_vec())
    }
}

http_endpoint::EndpointDef! {
    /// DELETE /v2/orders/{id} with the id sent verbatim (hyphenated).
    pub OrderCancelRaw(String),
    Ok => (), [
        /// cancel accepted.
        NO_CONTENT,
        /// some gateways answer 200.
        OK,
    ],
    Err => OrderCancelRawError, [
        /// no order with that id.
        NOT_FOUND => NotFound,
        /// the order can no longer be cancelled (already filled / cancelled).
        UNPROCESSABLE_ENTITY => NotCancelable,
    ],
    ConversionErr => serde_json::Error,
    ApiErr => apca::ApiError,

    fn method() -> http::Method {
        http::Method::DELETE
    }

    fn path(input: &Self::Input) -> std::borrow::Cow<'static, str> {
        format!("/v2/orders/{input}").into()
    }

    fn parse(_body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
        Ok(())
    }

    fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
        serde_json::from_slice::<Self::ApiError>(body).map_err(|_| body.to_vec())
    }
}

/// the one field we need from GET /v2/stocks/{symbol}/trades/latest (data api).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LatestTradeBody {
    pub trade: LatestTrade,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LatestTrade {
    #[serde(rename = "p")]
    pub price: f64,
}

http_endpoint::EndpointDef! {
    /// GET /v2/stocks/{symbol}/trades/latest?feed=iex on the data api.
    pub LatestTradeRaw(String),
    Ok => LatestTradeBody, [
        /// retrieved.
        OK,
    ],
    Err => LatestTradeRawError, [
        /// unknown symbol.
        NOT_FOUND => NotFound,
    ],
    ConversionErr => serde_json::Error,
    ApiErr => apca::ApiError,

    fn base_url() -> Option<std::borrow::Cow<'static, str>> {
        Some("https://data.alpaca.markets".into())
    }

    fn path(input: &Self::Input) -> std::borrow::Cow<'static, str> {
        format!("/v2/stocks/{input}/trades/latest").into()
    }

    fn query(_input: &Self::Input) -> Result<Option<std::borrow::Cow<'static, str>>, Self::ConversionError> {
        Ok(Some("feed=iex".into()))
    }

    fn parse(body: &[u8]) -> Result<Self::Output, Self::ConversionError> {
        serde_json::from_slice::<Self::Output>(body)
    }

    fn parse_err(body: &[u8]) -> Result<Self::ApiError, Vec<u8>> {
        serde_json::from_slice::<Self::ApiError>(body).map_err(|_| body.to_vec())
    }
}

// ── broker-agnostic order view + the fill-wait state machine ──

/// where an order stands, reduced to what the fill-wait needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderState {
    Filled,
    /// accepted / new / partially filled / pending — may still fill.
    Working,
    /// canceled / expired / rejected / … — will not fill any further.
    Terminal(String),
}

/// an order as the broker reports it (ids are the raw strings the broker returned).
#[derive(Debug, Clone)]
pub struct OrderView {
    pub id: String,
    pub client_order_id: String,
    pub symbol: String,
    pub state: OrderState,
    pub filled_qty: f64,
    pub avg_fill_price: Option<f64>,
    pub filled_at: Option<DateTime<Utc>>,
}

impl From<apca::api::v2::order::Order> for OrderView {
    fn from(o: apca::api::v2::order::Order) -> Self {
        use apca::api::v2::order::Status;
        let state = match o.status {
            Status::Filled => OrderState::Filled,
            Status::Canceled
            | Status::Expired
            | Status::Rejected
            | Status::Stopped
            | Status::Suspended => OrderState::Terminal(format!("{:?}", o.status)),
            _ => OrderState::Working,
        };
        Self {
            id: order_id_string(&o.id),
            client_order_id: o.client_order_id,
            symbol: o.symbol,
            state,
            filled_qty: num_to_f64(&o.filled_quantity),
            avg_fill_price: o.average_fill_price.as_ref().map(num_to_f64).filter(|p| *p > 0.0),
            filled_at: o.filled_at,
        }
    }
}

impl OrderView {
    /// the fill this order represents, if it filled at all (fully, or partially before
    /// ending in a terminal state — the broker holds those shares).
    fn fill(&self, ticker: &str, direction: TradeDirection) -> Option<OrderFill> {
        let filled = match self.state {
            OrderState::Filled => true,
            OrderState::Terminal(_) => self.filled_qty > 0.0,
            OrderState::Working => false,
        };
        if !filled {
            return None;
        }
        Some(OrderFill {
            ticker: ticker.to_string(),
            direction,
            quantity: self.filled_qty,
            fill_price: self.avg_fill_price.unwrap_or(0.0),
            filled_at: self.filled_at.unwrap_or_else(Utc::now),
        })
    }
}

/// the broker calls the fill-wait makes. `AlpacaBroker` implements it over the REST
/// api; tests implement it with scripted answers.
#[async_trait]
pub trait OrderApi: Send + Sync {
    async fn get_order(&self, id: &str) -> Result<OrderView, String>;
    async fn get_order_by_client_id(&self, client_order_id: &str) -> Result<OrderView, String>;
    async fn cancel_order(&self, id: &str) -> Result<(), String>;
    async fn positions(&self) -> Result<Vec<BrokerPosition>, String>;
}

/// what the order is for — decides what "the broker holds a position" means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Open,
    Close,
}

/// how long to wait for a market order to fill before giving up.
pub const FILL_TIMEOUT_MS: u64 = 15_000;
pub const FILL_POLL_MS: u64 = 250;
/// a failed poll is retried this many times (backoff `poll_ms * 2^n`) before the
/// fallback lookups run.
pub const POLL_RETRIES: u32 = 3;

#[derive(Debug, Clone, Copy)]
pub struct FillWait {
    pub timeout_ms: u64,
    pub poll_ms: u64,
    /// cancel a still-working order at the deadline (entries/closes) or leave it
    /// working (orphan close after hours: the point is to get flat).
    pub cancel_on_timeout: bool,
}

impl FillWait {
    pub const fn live() -> Self {
        Self { timeout_ms: FILL_TIMEOUT_MS, poll_ms: FILL_POLL_MS, cancel_on_timeout: true }
    }
}

async fn backoff(poll_ms: u64, attempt: u32) {
    if attempt > 0 {
        tokio::time::sleep(std::time::Duration::from_millis(poll_ms << attempt.min(4))).await;
    }
}

/// `get_order` with retries; `Err` carries the last error text.
async fn poll_with_retries<A: OrderApi + ?Sized>(api: &A, id: &str, poll_ms: u64) -> Result<OrderView, String> {
    let mut last = String::new();
    for attempt in 0..=POLL_RETRIES {
        backoff(poll_ms, attempt).await;
        match api.get_order(id).await {
            Ok(o) => return Ok(o),
            Err(e) => {
                warn!(order_id = id, attempt, error = %e, "order status poll failed");
                last = e;
            }
        }
    }
    Err(last)
}

async fn lookup_by_client_id<A: OrderApi + ?Sized>(api: &A, cid: &str, poll_ms: u64) -> Result<OrderView, String> {
    let mut last = String::new();
    for attempt in 0..=POLL_RETRIES {
        backoff(poll_ms, attempt).await;
        match api.get_order_by_client_id(cid).await {
            Ok(o) => return Ok(o),
            Err(e) => {
                warn!(client_order_id = cid, attempt, error = %e, "order lookup by client id failed");
                last = e;
            }
        }
    }
    Err(last)
}

/// what the fallbacks established after the id poll failed.
#[derive(Debug)]
enum Resolved {
    Fill(OrderFill),
    /// found and still working: keep waiting on this view.
    Continue(OrderView),
    Terminal(String),
    /// the broker could be asked and holds nothing for this order (yet).
    NotHeld,
    /// nothing could be asked.
    Unknown(String),
}

/// the broker's position in `ticker`, as a fill (open: adopt it; close: confirm flat).
fn position_as_fill(p: &BrokerPosition, direction: TradeDirection) -> OrderFill {
    OrderFill {
        ticker: p.ticker.clone(),
        direction,
        quantity: p.quantity,
        fill_price: p.entry_price,
        filled_at: Utc::now(),
    }
}

/// after the id poll failed: look the order up by client id (query string, no id in
/// the path), then ask the positions list. never concludes "not filled" from a failed
/// call.
async fn resolve_after_poll_failure<A: OrderApi + ?Sized>(
    api: &A,
    current: &OrderView,
    ticker: &str,
    direction: TradeDirection,
    intent: Intent,
    poll_ms: u64,
) -> Resolved {
    let last_err = match lookup_by_client_id(api, &current.client_order_id, poll_ms).await {
        Ok(o) => {
            if let Some(f) = o.fill(ticker, direction) {
                info!(order_id = %o.id, client_order_id = %o.client_order_id, fill_price = f.fill_price, quantity = f.quantity, "fill confirmed via client_order_id lookup");
                return Resolved::Fill(f);
            }
            return match &o.state {
                OrderState::Terminal(s) => Resolved::Terminal(s.clone()),
                _ => Resolved::Continue(o),
            };
        }
        Err(e) => e,
    };
    match api.positions().await {
        Ok(list) => {
            let held = list.iter().find(|p| p.ticker == ticker);
            match (intent, held) {
                (Intent::Open, Some(p)) if p.direction == direction => {
                    warn!(ticker, quantity = p.quantity, entry_price = p.entry_price, "fill inferred from the broker's position (order lookups failed)");
                    Resolved::Fill(position_as_fill(p, direction))
                }
                (Intent::Close, None) => {
                    warn!(ticker, "close inferred from the broker being flat (order lookups failed); fill price unknown");
                    Resolved::Fill(OrderFill {
                        ticker: ticker.to_string(),
                        direction,
                        quantity: current.filled_qty,
                        fill_price: 0.0,
                        filled_at: Utc::now(),
                    })
                }
                _ => Resolved::NotHeld,
            }
        }
        Err(e) => Resolved::Unknown(format!("order lookup: {last_err}; positions: {e}")),
    }
}

/// poll `initial` until it fills or ends. a failed poll is retried, then resolved
/// through the client-id lookup and the positions list; at the deadline a working order
/// is cancelled (when `cfg.cancel_on_timeout`) and its final state re-read. `Err` means
/// the broker holds nothing for this order — or, as `Unconfirmed`, that the broker
/// could not be asked at all (the caller must reconcile).
pub async fn wait_for_fill_with<A: OrderApi + ?Sized>(
    api: &A,
    initial: OrderView,
    ticker: &str,
    direction: TradeDirection,
    intent: Intent,
    cfg: FillWait,
) -> Result<OrderFill, BrokerError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(cfg.timeout_ms);
    let mut current = initial;
    // once the id path has failed, poll by client id instead
    let mut by_client_id = false;
    let mut last_unknown: Option<String> = None;
    loop {
        if let Some(f) = current.fill(ticker, direction) {
            return Ok(f);
        }
        if let OrderState::Terminal(s) = &current.state {
            return Err(BrokerError::OrderRejected(format!("order {} ended in status {s}", current.id)));
        }
        if std::time::Instant::now() >= deadline {
            return finalize_timeout(api, &current, ticker, direction, intent, cfg, last_unknown).await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(cfg.poll_ms)).await;

        let polled = if by_client_id {
            lookup_by_client_id(api, &current.client_order_id, cfg.poll_ms).await
        } else {
            poll_with_retries(api, &current.id, cfg.poll_ms).await
        };
        match polled {
            Ok(o) => {
                current = o;
                continue;
            }
            Err(e) => {
                error!(order_id = %current.id, client_order_id = %current.client_order_id, error = %e, "order poll exhausted its retries — resolving through fallbacks");
            }
        }
        by_client_id = true;
        match resolve_after_poll_failure(api, &current, ticker, direction, intent, cfg.poll_ms).await {
            Resolved::Fill(f) => return Ok(f),
            Resolved::Continue(o) => current = o,
            Resolved::Terminal(s) => {
                return Err(BrokerError::OrderRejected(format!("order {} ended in status {s}", current.id)));
            }
            Resolved::NotHeld => {}
            Resolved::Unknown(e) => last_unknown = Some(e),
        }
    }
}

/// the deadline passed with the order still working: cancel (idempotent — a fill that
/// raced the cancel shows up in the re-read), then establish the final state.
async fn finalize_timeout<A: OrderApi + ?Sized>(
    api: &A,
    current: &OrderView,
    ticker: &str,
    direction: TradeDirection,
    intent: Intent,
    cfg: FillWait,
    last_unknown: Option<String>,
) -> Result<OrderFill, BrokerError> {
    if !cfg.cancel_on_timeout {
        return Err(BrokerError::OrderWorking(current.id.clone()));
    }
    match api.cancel_order(&current.id).await {
        Ok(()) => info!(order_id = %current.id, "cancel requested after fill timeout"),
        Err(e) => warn!(order_id = %current.id, error = %e, "cancel request failed (order may have filled) — re-reading"),
    }
    // the cancel needs a moment to settle; then the order's final word
    tokio::time::sleep(std::time::Duration::from_millis(cfg.poll_ms)).await;
    let lookup_err = match lookup_by_client_id(api, &current.client_order_id, cfg.poll_ms).await {
        Ok(o) => {
            if let Some(f) = o.fill(ticker, direction) {
                warn!(order_id = %o.id, quantity = f.quantity, fill_price = f.fill_price, "order filled before the cancel took effect");
                return Ok(f);
            }
            if matches!(o.state, OrderState::Terminal(_)) && intent == Intent::Open {
                // confirmed unfilled — but only trust it if the broker agrees it holds nothing
                if let Ok(list) = api.positions().await {
                    if let Some(p) = list.iter().find(|p| p.ticker == ticker && p.direction == direction) {
                        warn!(ticker, quantity = p.quantity, "order reads cancelled but the broker holds the position — adopting");
                        return Ok(position_as_fill(p, direction));
                    }
                }
                return Err(BrokerError::OrderRejected(format!(
                    "order {} not filled within {}ms (status {:?}); cancelled",
                    current.id, cfg.timeout_ms, o.state
                )));
            }
            if matches!(o.state, OrderState::Terminal(_)) {
                return Err(BrokerError::OrderRejected(format!(
                    "order {} not filled within {}ms (status {:?}); cancelled",
                    current.id, cfg.timeout_ms, o.state
                )));
            }
            // still working after a cancel: cannot be trusted either way
            return Err(BrokerError::Unconfirmed(format!(
                "order {} still {:?} after cancel at the {}ms deadline",
                current.id, o.state, cfg.timeout_ms
            )));
        }
        Err(e) => e,
    };
    match api.positions().await {
        Ok(list) => {
            let held = list.iter().find(|p| p.ticker == ticker);
            match (intent, held) {
                (Intent::Open, Some(p)) if p.direction == direction => Ok(position_as_fill(p, direction)),
                (Intent::Close, None) => Ok(OrderFill {
                    ticker: ticker.to_string(),
                    direction,
                    quantity: current.filled_qty,
                    fill_price: 0.0,
                    filled_at: Utc::now(),
                }),
                _ => Err(BrokerError::OrderRejected(format!(
                    "order {} not filled within {}ms (lookups failed: {lookup_err}); cancelled; broker holds no position",
                    current.id, cfg.timeout_ms
                ))),
            }
        }
        Err(e) => Err(BrokerError::Unconfirmed(format!(
            "order {} state unknown at the {}ms deadline: {lookup_err}; positions: {e}{}",
            current.id,
            cfg.timeout_ms,
            last_unknown.map(|u| format!("; earlier: {u}")).unwrap_or_default()
        ))),
    }
}

// ── alpaca ──

/// alpaca paper trading broker. uses the apca crate for order execution.
pub struct AlpacaBroker {
    client: apca::Client,
    /// book tag in every `client_order_id` (`gtf-<tag>-<ticker>-<ms>`).
    tag: String,
}

impl AlpacaBroker {
    pub fn new(api_key: String, api_secret: String) -> Result<Self, BrokerError> {
        let api_info = apca::ApiInfo::from_parts(
            "https://paper-api.alpaca.markets",
            &api_key,
            &api_secret,
        )
        .map_err(|e| BrokerError::ConnectionError(format!("invalid credentials: {e}")))?;

        Ok(Self {
            client: apca::Client::new(api_info),
            tag: "primary".to_string(),
        })
    }

    /// the book name stamped into `client_order_id`.
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tag = tag.to_string();
        self
    }

    /// last trade (iex feed) from the data api — the reference for after-hours limits.
    pub async fn latest_trade_price(&self, symbol: &str) -> Result<f64, BrokerError> {
        self.client
            .issue::<LatestTradeRaw>(&symbol.to_string())
            .await
            .map(|b| b.trade.price)
            .map_err(|e| BrokerError::ConnectionError(format!("latest trade {symbol}: {e:?}")))
    }

    /// submit a market order and wait for the fill (see [`wait_for_fill_with`]).
    #[doc(hidden)]
    pub async fn submit_market_order_with_timeout(
        &self,
        ticker: &str,
        direction: TradeDirection,
        shares: i64,
        timeout_ms: u64,
    ) -> Result<OrderFill, BrokerError> {
        use apca::api::v2::order;

        let side = match direction {
            TradeDirection::Long => order::Side::Buy,
            TradeDirection::Short => order::Side::Sell,
        };
        let cid = client_order_id(&self.tag, ticker, Utc::now().timestamp_millis());
        let req = order::CreateReqInit {
            type_: order::Type::Market,
            time_in_force: order::TimeInForce::Day,
            client_order_id: Some(cid.clone()),
            ..Default::default()
        }
        .init(ticker, side, order::Amount::quantity(shares));

        let created = self
            .client
            .issue::<order::Create>(&req)
            .await
            .map_err(|e| BrokerError::OrderRejected(format!("{e:?}")))?;
        let view = OrderView::from(created);
        info!(
            ticker,
            ?direction,
            shares,
            order_id = %view.id,
            client_order_id = %view.client_order_id,
            status = ?view.state,
            "order submitted (polling by the id exactly as returned)"
        );
        debug_assert_eq!(view.client_order_id, cid);

        let cfg = FillWait { timeout_ms, ..FillWait::live() };
        let fill = wait_for_fill_with(self, view, ticker, direction, Intent::Open, cfg).await?;
        if fill.fill_price <= 0.0 {
            warn!(ticker, quantity = fill.quantity, "fill reported without an average price");
        }
        Ok(fill)
    }
}

#[async_trait]
impl OrderApi for AlpacaBroker {
    async fn get_order(&self, id: &str) -> Result<OrderView, String> {
        debug!(order_id = id, path = %format!("/v2/orders/{id}"), "GET order");
        self.client
            .issue::<OrderGetRaw>(&id.to_string())
            .await
            .map(OrderView::from)
            .map_err(|e| format!("{e:?}"))
    }

    async fn get_order_by_client_id(&self, client_order_id: &str) -> Result<OrderView, String> {
        use apca::api::v2::order;
        self.client
            .issue::<order::GetByClientId>(&client_order_id.to_string())
            .await
            .map(OrderView::from)
            .map_err(|e| format!("{e:?}"))
    }

    async fn cancel_order(&self, id: &str) -> Result<(), String> {
        self.client
            .issue::<OrderCancelRaw>(&id.to_string())
            .await
            .map_err(|e| format!("{e:?}"))
    }

    async fn positions(&self) -> Result<Vec<BrokerPosition>, String> {
        self.open_positions().await.map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Broker for AlpacaBroker {
    fn kind(&self) -> &'static str {
        "alpaca_paper"
    }

    async fn submit_order(
        &self,
        ticker: &str,
        direction: TradeDirection,
        quantity: f64,
    ) -> Result<OrderFill, BrokerError> {
        // truncate to whole shares for alpaca
        let shares = quantity.floor() as i64;
        if shares <= 0 {
            return Err(BrokerError::OrderRejected(
                "quantity must be at least 1 share".to_string(),
            ));
        }
        self.submit_market_order_with_timeout(ticker, direction, shares, FILL_TIMEOUT_MS)
            .await
    }

    async fn open_positions(&self) -> Result<Vec<BrokerPosition>, BrokerError> {
        use apca::api::v2::{position, positions};

        let list = self
            .client
            .issue::<positions::List>(&())
            .await
            .map_err(|e| BrokerError::ConnectionError(format!("{e:?}")))?;

        Ok(list
            .into_iter()
            .map(|p| BrokerPosition {
                ticker: p.symbol.clone(),
                direction: match p.side {
                    position::Side::Long => TradeDirection::Long,
                    position::Side::Short => TradeDirection::Short,
                },
                quantity: num_to_f64(&p.quantity).abs(),
                entry_price: num_to_f64(&p.average_entry_price),
                current_price: p.current_price.as_ref().map(num_to_f64).filter(|x| *x > 0.0),
            })
            .collect())
    }

    async fn close_position(&self, ticker: &str) -> Result<OrderFill, BrokerError> {
        use apca::api::v2::position;

        let symbol = apca::api::v2::asset::Symbol::Sym(ticker.to_string());

        let order = self
            .client
            .issue::<position::Delete>(&symbol)
            .await
            .map_err(|e| {
                // check if the error is a NotFound
                let msg = format!("{e:?}");
                if msg.contains("404") || msg.contains("not found") || msg.contains("NotFound") {
                    BrokerError::NoPosition(ticker.to_string())
                } else {
                    BrokerError::ConnectionError(msg)
                }
            })?;

        // the closing order side tells us the close direction
        let direction = match order.side {
            apca::api::v2::order::Side::Buy => TradeDirection::Long,
            apca::api::v2::order::Side::Sell => TradeDirection::Short,
        };
        let view = OrderView::from(order);
        info!(ticker, order_id = %view.id, client_order_id = %view.client_order_id, "close order submitted");

        // a close that does not fill leaves the position open at the broker —
        // surface that loudly rather than pretending it closed.
        wait_for_fill_with(self, view, ticker, direction, Intent::Close, FillWait::live()).await
    }

    async fn close_orphan(&self, pos: &BrokerPosition, market_open: bool) -> Result<OrphanClose, BrokerError> {
        use apca::api::v2::order;

        if market_open {
            return self.close_position(&pos.ticker).await.map(OrphanClose::Filled);
        }
        let last = match self.latest_trade_price(&pos.ticker).await {
            Ok(p) if p > 0.0 => p,
            other => {
                warn!(ticker = %pos.ticker, result = ?other, "no latest trade; using the position's mark");
                pos.current_price.filter(|p| *p > 0.0).unwrap_or(pos.entry_price)
            }
        };
        let (side, limit, close_direction) = match pos.direction {
            TradeDirection::Long => (order::Side::Sell, last * (1.0 - ORPHAN_LIMIT_PCT), TradeDirection::Short),
            TradeDirection::Short => (order::Side::Buy, last * (1.0 + ORPHAN_LIMIT_PCT), TradeDirection::Long),
        };
        let limit = (limit * 100.0).round() / 100.0;
        let shares = pos.quantity.floor() as i64;
        if shares <= 0 {
            return Err(BrokerError::OrderRejected(format!("orphan {} has no whole shares", pos.ticker)));
        }
        let cid = client_order_id(&format!("{}-orphan", self.tag), &pos.ticker, Utc::now().timestamp_millis());
        let req = order::CreateReqInit {
            type_: order::Type::Limit,
            time_in_force: order::TimeInForce::Day,
            limit_price: Some(num_from_price(limit)?),
            extended_hours: true,
            client_order_id: Some(cid),
            ..Default::default()
        }
        .init(&pos.ticker, side, order::Amount::quantity(shares));
        let created = self
            .client
            .issue::<order::Create>(&req)
            .await
            .map_err(|e| BrokerError::OrderRejected(format!("{e:?}")))?;
        let view = OrderView::from(created);
        warn!(ticker = %pos.ticker, shares, limit, last, order_id = %view.id, client_order_id = %view.client_order_id, "orphan close: extended-hours day limit submitted");
        let cfg = FillWait { cancel_on_timeout: false, ..FillWait::live() };
        match wait_for_fill_with(self, view, &pos.ticker, close_direction, Intent::Close, cfg).await {
            Ok(fill) => Ok(OrphanClose::Filled(fill)),
            Err(BrokerError::OrderWorking(order_id)) => Ok(OrphanClose::Working { order_id, limit_price: limit }),
            Err(e) => Err(e),
        }
    }
}

/// after-hours orphan close: limit at the last trade ± this fraction.
pub const ORPHAN_LIMIT_PCT: f64 = 0.003;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn simulated_fill_at_last_price_no_slippage() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(150.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Long, 100.0)
            .await
            .unwrap();

        assert_eq!(fill.ticker, "SPY");
        assert!((fill.fill_price - 150.0).abs() < f64::EPSILON);
        assert!((fill.quantity - 100.0).abs() < f64::EPSILON);
        assert!(matches!(fill.direction, TradeDirection::Long));
    }

    #[tokio::test]
    async fn simulated_long_slippage_increases_price() {
        let broker = SimulatedBroker::new(10.0); // 10 bps = 0.1%
        broker.set_last_price(100.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Long, 50.0)
            .await
            .unwrap();

        // 100.0 + 100.0 * 10 / 10000 = 100.10
        assert!((fill.fill_price - 100.10).abs() < 1e-10);
    }

    #[tokio::test]
    async fn simulated_short_slippage_decreases_price() {
        let broker = SimulatedBroker::new(10.0); // 10 bps
        broker.set_last_price(100.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Short, 50.0)
            .await
            .unwrap();

        // 100.0 - 100.0 * 10 / 10000 = 99.90
        assert!((fill.fill_price - 99.90).abs() < 1e-10);
    }

    #[tokio::test]
    async fn simulated_close_position() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(100.0);

        broker
            .submit_order("SPY", TradeDirection::Long, 50.0)
            .await
            .unwrap();

        broker.set_last_price(105.0);
        let close = broker.close_position("SPY").await.unwrap();
        assert!((close.fill_price - 105.0).abs() < f64::EPSILON);
        assert!(matches!(close.direction, TradeDirection::Short));
    }

    #[tokio::test]
    async fn simulated_open_positions_reflects_state() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(100.0);
        assert!(broker.open_positions().await.unwrap().is_empty());
        broker.submit_order("SPY", TradeDirection::Long, 5.0).await.unwrap();
        let open = broker.open_positions().await.unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].ticker, "SPY");
        assert!((open[0].quantity - 5.0).abs() < f64::EPSILON);
        broker.close_position("SPY").await.unwrap();
        assert!(broker.open_positions().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn simulated_close_no_position_errors() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(100.0);

        let result = broker.close_position("SPY").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn simulated_no_price_set_errors() {
        let broker = SimulatedBroker::new(0.0);
        let result = broker
            .submit_order("SPY", TradeDirection::Long, 50.0)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn simulated_direction_handling() {
        let broker = SimulatedBroker::new(5.0); // 5 bps
        broker.set_last_price(200.0);

        let long_fill = broker
            .submit_order("QQQ", TradeDirection::Long, 10.0)
            .await
            .unwrap();
        assert!(long_fill.fill_price > 200.0);

        // close long → direction is Short
        broker.set_last_price(210.0);
        let close = broker.close_position("QQQ").await.unwrap();
        assert!(matches!(close.direction, TradeDirection::Short));
        // closing a long means selling (Short slippage direction) → price decreases
        assert!(close.fill_price < 210.0);
    }

    #[tokio::test]
    async fn simulated_per_ticker_prices_fill_each_ticker_at_its_own_price() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price_for("AAPL", 200.0);
        broker.set_last_price_for("NVDA", 100.0);
        let a = broker.submit_order("AAPL", TradeDirection::Long, 1.0).await.unwrap();
        let n = broker.submit_order("NVDA", TradeDirection::Short, 1.0).await.unwrap();
        assert!((a.fill_price - 200.0).abs() < f64::EPSILON);
        assert!((n.fill_price - 100.0).abs() < f64::EPSILON);
        // the last-arrived ticker's price no longer leaks into another ticker's close
        broker.set_last_price_for("NVDA", 90.0);
        let close_a = broker.close_position("AAPL").await.unwrap();
        assert!((close_a.fill_price - 200.0).abs() < f64::EPSILON, "{}", close_a.fill_price);
        let close_n = broker.close_position("NVDA").await.unwrap();
        assert!((close_n.fill_price - 90.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn simulated_falls_back_to_global_price_without_a_ticker_price() {
        let broker = SimulatedBroker::new(0.0);
        broker.set_last_price(50.0);
        let f = broker.submit_order("MSFT", TradeDirection::Long, 1.0).await.unwrap();
        assert!((f.fill_price - 50.0).abs() < f64::EPSILON);
        // set_last_price_for also refreshes the global fallback
        broker.set_last_price_for("AAPL", 70.0);
        let g = broker.submit_order("AMZN", TradeDirection::Long, 1.0).await.unwrap();
        assert!((g.fill_price - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn broker_kinds() {
        assert_eq!(SimulatedBroker::new(0.0).kind(), "simulated");
        let b = AlpacaBroker::new("key".to_string(), "secret".to_string()).unwrap();
        assert_eq!(b.kind(), "alpaca_paper");
    }

    #[test]
    fn alpaca_broker_construction() {
        let broker = AlpacaBroker::new("key".to_string(), "secret".to_string());
        assert!(broker.is_ok());
    }

    #[test]
    fn alpaca_set_last_price_noop() {
        let broker = AlpacaBroker::new("key".to_string(), "secret".to_string()).unwrap();
        // default trait method — should not panic
        broker.set_last_price(150.0);
    }

    #[tokio::test]
    async fn simulated_set_last_price_via_trait() {
        let broker: Box<dyn Broker> = Box::new(SimulatedBroker::new(0.0));
        broker.set_last_price(200.0);

        let fill = broker
            .submit_order("SPY", TradeDirection::Long, 10.0)
            .await
            .unwrap();
        assert!((fill.fill_price - 200.0).abs() < f64::EPSILON);
    }

    // ── client_order_id / id formatting ──

    #[test]
    fn client_order_id_format_and_sanitising() {
        let id = client_order_id("primary", "AMZN", 1_760_535_420_936);
        assert_eq!(id, "gtf-primary-AMZN-1760535420936");
        // a shadow-style name with ':' and a ticker with '/' are cleaned, never rejected
        let id = client_order_id("shadow:vpin 0.26", "BRK/B", 1);
        assert_eq!(id, "gtf-shadow-vpin-0.26-BRK-B-1");
        assert!(id.len() <= 128);
        let long = client_order_id(&"x".repeat(200), "AAPL", 1);
        assert_eq!(long.len(), 128);
    }

    /// the id we send must be the hyphenated form alpaca returned — never apca's
    /// `as_simple()` path (which alpaca rejects with 422 `invalid order_id`).
    #[test]
    fn order_id_round_trips_hyphenated_into_the_path() {
        use http_endpoint::Endpoint;
        let raw = "4f807b12-2af2-49ad-952b-c39c2b212234";
        let id = apca::api::v2::order::Id(uuid::Uuid::parse_str(raw).unwrap());
        assert_eq!(order_id_string(&id), raw);
        assert_eq!(OrderGetRaw::path(&order_id_string(&id)), format!("/v2/orders/{raw}"));
        assert_eq!(OrderCancelRaw::path(&order_id_string(&id)), format!("/v2/orders/{raw}"));
        assert_eq!(OrderCancelRaw::method(), http::Method::DELETE);
        // apca 0.30's own path is the un-hyphenated one — the bug this guards against
        assert_eq!(
            apca::api::v2::order::Get::path(&id),
            "/v2/orders/4f807b122af249ad952bc39c2b212234"
        );
        let view: OrderView = serde_json::from_str::<apca::api::v2::order::Order>(&format!(
            r#"{{"id":"{raw}","client_order_id":"gtf-primary-NVDA-1","created_at":"2026-10-15T13:50:00.387495Z",
               "updated_at":null,"submitted_at":null,"filled_at":"2026-10-15T13:50:01.5Z","expired_at":null,"canceled_at":null,
               "asset_class":"us_equity","asset_id":"b0b6dd9d-8b9b-48a9-ba46-b9d54906e415","symbol":"NVDA","qty":"14",
               "filled_qty":"14","type":"market","order_class":"simple","side":"sell","time_in_force":"day",
               "limit_price":null,"stop_price":null,"trail_price":null,"trail_percent":null,"filled_avg_price":"214.23",
               "status":"filled","extended_hours":false,"legs":null}}"#
        ))
        .unwrap()
        .into();
        assert_eq!(view.id, raw);
        assert_eq!(view.state, OrderState::Filled);
        assert_eq!(view.client_order_id, "gtf-primary-NVDA-1");
        let f = view.fill("NVDA", TradeDirection::Short).unwrap();
        assert!((f.fill_price - 214.23).abs() < 1e-9);
        assert!((f.quantity - 14.0).abs() < 1e-9);
    }

    // ── the poll-failure decision tree, against a scripted broker ──

    use std::sync::Mutex as StdMutex;

    struct Script {
        /// answers to `get_order`, consumed in order; empty → the last one repeats.
        get: Vec<Result<OrderView, String>>,
        by_cid: Vec<Result<OrderView, String>>,
        positions: Vec<Result<Vec<BrokerPosition>, String>>,
        cancel: Result<(), String>,
        /// once `cancel_order` ran, `get_order_by_client_id` answers this instead.
        after_cancel: Option<OrderView>,
    }

    impl Default for Script {
        fn default() -> Self {
            Self { get: vec![], by_cid: vec![], positions: vec![], cancel: Ok(()), after_cancel: None }
        }
    }

    struct MockApi {
        script: StdMutex<Script>,
        calls: StdMutex<Vec<String>>,
    }

    impl MockApi {
        fn new(script: Script) -> Self {
            Self { script: StdMutex::new(script), calls: StdMutex::new(Vec::new()) }
        }
        fn next<T: Clone>(v: &mut Vec<T>) -> Option<T> {
            if v.len() > 1 { Some(v.remove(0)) } else { v.first().cloned() }
        }
        fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl OrderApi for MockApi {
        async fn get_order(&self, id: &str) -> Result<OrderView, String> {
            self.calls.lock().unwrap().push(format!("get:{id}"));
            Self::next(&mut self.script.lock().unwrap().get).unwrap_or(Err("unscripted".into()))
        }
        async fn get_order_by_client_id(&self, cid: &str) -> Result<OrderView, String> {
            self.calls.lock().unwrap().push(format!("cid:{cid}"));
            Self::next(&mut self.script.lock().unwrap().by_cid).unwrap_or(Err("unscripted".into()))
        }
        async fn cancel_order(&self, id: &str) -> Result<(), String> {
            self.calls.lock().unwrap().push(format!("cancel:{id}"));
            let mut s = self.script.lock().unwrap();
            if let Some(v) = s.after_cancel.take() {
                s.by_cid = vec![Ok(v)];
            }
            s.cancel.clone()
        }
        async fn positions(&self) -> Result<Vec<BrokerPosition>, String> {
            self.calls.lock().unwrap().push("positions".to_string());
            Self::next(&mut self.script.lock().unwrap().positions).unwrap_or(Err("unscripted".into()))
        }
    }

    fn view(state: OrderState, filled_qty: f64, px: Option<f64>) -> OrderView {
        OrderView {
            id: "4f807b12-2af2-49ad-952b-c39c2b212234".into(),
            client_order_id: "gtf-primary-AMZN-1".into(),
            symbol: "AMZN".into(),
            state,
            filled_qty,
            avg_fill_price: px,
            filled_at: None,
        }
    }

    fn held(ticker: &str, direction: TradeDirection, qty: f64, px: f64) -> BrokerPosition {
        BrokerPosition { ticker: ticker.into(), direction, quantity: qty, entry_price: px, current_price: None }
    }

    const FAST: FillWait = FillWait { timeout_ms: 400, poll_ms: 1, cancel_on_timeout: true };
    const E422: &str = "Endpoint(UnexpectedStatus(422, Ok(ApiError { message: \"invalid order_id\" })))";

    #[tokio::test]
    async fn poll_error_then_retry_succeeds() {
        let api = MockApi::new(Script {
            get: vec![Err(E422.into()), Ok(view(OrderState::Filled, 12.0, Some(249.72)))],
            ..Default::default()
        });
        let f = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST)
            .await
            .unwrap();
        assert!((f.fill_price - 249.72).abs() < 1e-9);
        assert_eq!(api.calls().iter().filter(|c| c.starts_with("get:")).count(), 2);
    }

    /// today's failure: every id poll is 422 — the fill must still come back, via the
    /// client_order_id lookup, and nothing may be cancelled.
    #[tokio::test]
    async fn persistent_poll_error_resolves_through_client_order_id() {
        let api = MockApi::new(Script {
            get: vec![Err(E422.into())],
            by_cid: vec![Ok(view(OrderState::Filled, 12.0, Some(249.72)))],
            ..Default::default()
        });
        let f = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST)
            .await
            .expect("fill via client id");
        assert!((f.fill_price - 249.72).abs() < 1e-9);
        assert!((f.quantity - 12.0).abs() < 1e-9);
        let calls = api.calls();
        assert_eq!(calls.iter().filter(|c| c.starts_with("get:")).count(), 1 + POLL_RETRIES as usize);
        assert!(calls.iter().any(|c| c == "cid:gtf-primary-AMZN-1"));
        assert!(!calls.iter().any(|c| c.starts_with("cancel")), "{calls:?}");
    }

    #[tokio::test]
    async fn poll_and_lookup_fail_but_the_broker_holds_the_position() {
        let api = MockApi::new(Script {
            get: vec![Err(E422.into())],
            by_cid: vec![Err("timeout".into())],
            positions: vec![Ok(vec![held("AMZN", TradeDirection::Short, 12.0, 249.70)])],
            ..Default::default()
        });
        let f = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST)
            .await
            .expect("fill inferred from the position");
        assert!((f.fill_price - 249.70).abs() < 1e-9);
        assert!((f.quantity - 12.0).abs() < 1e-9);
        assert!(!api.calls().iter().any(|c| c.starts_with("cancel")));
    }

    #[tokio::test]
    async fn confirmed_unfilled_is_cancelled_and_errs() {
        // the broker answers: the order is working and there is no position → at the
        // deadline it is cancelled, re-read as canceled, and Err comes back
        let api = MockApi::new(Script {
            get: vec![Err(E422.into())],
            by_cid: vec![Ok(view(OrderState::Working, 0.0, None))],
            positions: vec![Ok(vec![])],
            after_cancel: Some(view(OrderState::Terminal("Canceled".into()), 0.0, None)),
            ..Default::default()
        });
        let r = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST).await;
        let calls = api.calls();
        assert!(calls.iter().any(|c| c.starts_with("cancel:4f807b12-2af2-49ad-952b-c39c2b212234")), "{calls:?}");
        match r {
            Err(BrokerError::OrderRejected(msg)) => assert!(msg.contains("cancelled"), "{msg}"),
            other => panic!("expected OrderRejected, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fill_that_raced_the_cancel_is_returned() {
        let api = MockApi::new(Script {
            get: vec![Ok(view(OrderState::Working, 0.0, None))],
            by_cid: vec![Ok(view(OrderState::Working, 0.0, None))],
            cancel: Err("422 not cancelable".into()),
            after_cancel: Some(view(OrderState::Filled, 12.0, Some(249.5))),
            ..Default::default()
        });
        let f = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST)
            .await
            .expect("the fill wins over the cancel");
        assert!((f.fill_price - 249.5).abs() < 1e-9);
        assert!(api.calls().iter().any(|c| c.starts_with("cancel")));
    }

    #[tokio::test]
    async fn partial_fill_before_cancel_is_a_fill_for_the_filled_shares() {
        let api = MockApi::new(Script {
            get: vec![Ok(view(OrderState::Working, 0.0, None))],
            by_cid: vec![Ok(view(OrderState::Working, 0.0, None))],
            after_cancel: Some(view(OrderState::Terminal("Canceled".into()), 5.0, Some(249.5))),
            ..Default::default()
        });
        let f = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST)
            .await
            .unwrap();
        assert!((f.quantity - 5.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn terminal_status_is_rejected_without_cancel() {
        let api = MockApi::new(Script {
            get: vec![Ok(view(OrderState::Terminal("Rejected".into()), 0.0, None))],
            ..Default::default()
        });
        let r = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST).await;
        assert!(matches!(r, Err(BrokerError::OrderRejected(_))), "{r:?}");
        assert!(!api.calls().iter().any(|c| c.starts_with("cancel")));
    }

    #[tokio::test]
    async fn nothing_reachable_is_unconfirmed_never_rejected() {
        let api = MockApi::new(Script {
            get: vec![Err("connection reset".into())],
            by_cid: vec![Err("connection reset".into())],
            positions: vec![Err("connection reset".into())],
            cancel: Err("connection reset".into()),
            after_cancel: None,
        });
        let r = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Short, Intent::Open, FAST).await;
        assert!(matches!(r, Err(BrokerError::Unconfirmed(_))), "{r:?}");
    }

    #[tokio::test]
    async fn close_intent_treats_a_flat_broker_as_closed() {
        let api = MockApi::new(Script {
            get: vec![Err(E422.into())],
            by_cid: vec![Err("timeout".into())],
            positions: vec![Ok(vec![held("NVDA", TradeDirection::Short, 14.0, 214.0)])],
            ..Default::default()
        });
        let f = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AMZN", TradeDirection::Long, Intent::Close, FAST)
            .await
            .expect("AMZN is flat at the broker → closed");
        assert_eq!(f.fill_price, 0.0, "price unknown is reported as 0 (caller treats as unknown)");
    }

    #[tokio::test]
    async fn no_cancel_on_timeout_reports_the_working_order() {
        let api = MockApi::new(Script {
            get: vec![Ok(view(OrderState::Working, 0.0, None))],
            ..Default::default()
        });
        let cfg = FillWait { cancel_on_timeout: false, ..FAST };
        let r = wait_for_fill_with(&api, view(OrderState::Working, 0.0, None), "AAPL", TradeDirection::Short, Intent::Close, cfg).await;
        assert!(matches!(r, Err(BrokerError::OrderWorking(ref id)) if id == "4f807b12-2af2-49ad-952b-c39c2b212234"), "{r:?}");
        assert!(!api.calls().iter().any(|c| c.starts_with("cancel")));
    }

    /// live, after hours (16:00–20:00 ET or 04:00–09:30 ET): buy 1 AAPL with an
    /// extended-hours day limit at the last trade +0.3 %, wait for the fill through the
    /// fixed poll path, close it the same way, and require the account to end flat.
    /// during regular hours it uses market orders. a resting order that does not fill
    /// within the timeout is cancelled and the test fails loudly.
    /// run: cargo test -p data_feed -- --ignored live_order_roundtrip --nocapture
    #[tokio::test]
    #[ignore]
    async fn live_order_roundtrip() {
        use apca::api::v2::order;
        let _ = dotenvy::dotenv();
        let broker = AlpacaBroker::new(
            std::env::var("APCA_API_KEY_ID").unwrap(),
            std::env::var("APCA_API_SECRET_KEY").unwrap(),
        )
        .unwrap()
        .with_tag("roundtrip");
        let sym = "AAPL";
        let before = broker.open_positions().await.unwrap();
        assert!(before.iter().all(|p| p.ticker != sym), "{sym} already held: {before:?}");

        let market_open = crate::session_clock::is_regular_hours(Utc::now());
        let fill = if market_open {
            broker.submit_market_order_with_timeout(sym, TradeDirection::Long, 1, FILL_TIMEOUT_MS).await
        } else {
            let last = broker.latest_trade_price(sym).await.unwrap();
            let limit = ((last * (1.0 + ORPHAN_LIMIT_PCT)) * 100.0).round() / 100.0;
            let cid = client_order_id("roundtrip", sym, Utc::now().timestamp_millis());
            let req = order::CreateReqInit {
                type_: order::Type::Limit,
                time_in_force: order::TimeInForce::Day,
                limit_price: Some(num_from_price(limit).unwrap()),
                extended_hours: true,
                client_order_id: Some(cid),
                ..Default::default()
            }
            .init(sym, order::Side::Buy, order::Amount::quantity(1));
            let created = broker.client.issue::<order::Create>(&req).await.unwrap();
            let v = OrderView::from(created);
            println!("open order: id={} client_order_id={} state={:?} last={last} limit={limit}", v.id, v.client_order_id, v.state);
            let cfg = FillWait { timeout_ms: 60_000, ..FillWait::live() };
            wait_for_fill_with(&broker, v, sym, TradeDirection::Long, Intent::Open, cfg).await
        };
        println!("open result: {fill:?}");
        let fill = fill.expect("the fixed poll path returns the fill");
        assert!(fill.fill_price > 0.0);

        let pos = broker.open_positions().await.unwrap().into_iter().find(|p| p.ticker == sym).expect("position held");
        println!("position: {pos:?}");
        let closed = broker.close_orphan(&pos, market_open).await;
        println!("close result: {closed:?}");
        match closed.expect("close") {
            OrphanClose::Filled(f) => assert!(f.quantity >= 1.0),
            OrphanClose::Working { order_id, .. } => {
                // give it a little longer, then cancel so the account is flat
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                let o = broker.get_order(&order_id).await.unwrap();
                if o.state != OrderState::Filled {
                    let _ = broker.cancel_order(&order_id).await;
                    panic!("close limit did not fill ({:?}); cancelled — CLOSE {sym} BY HAND", o.state);
                }
            }
        }
        let after = broker.open_positions().await.unwrap();
        assert!(after.iter().all(|p| p.ticker != sym), "{sym} position left open: {after:?}");
    }
}
