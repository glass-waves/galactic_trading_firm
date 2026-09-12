use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Candle {
    pub timestamp: DateTime<Utc>,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Timescale {
    OneMinute,
    FiveMinute,
    OneHour,
    OneDay,
    OneMonth,
}

/// position context exposed to the indicator pipeline.
#[derive(Debug, Clone)]
pub struct PositionContext {
    /// +1.0 = long, -1.0 = short, 0.0 = flat
    pub direction: f64,
    /// unrealized P&L as percentage of entry price
    pub unrealized_pnl_pct: f64,
    /// milliseconds since position entry
    pub hold_duration_ms: i64,
    /// configured maximum hold time in milliseconds
    pub max_hold_ms: i64,
}

/// snapshot of current market state across all timescales.
/// rebuilt every tick from the price feed.
#[derive(Debug, Clone)]
pub struct MarketState {
    /// current tick price
    pub last_price: f64,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: DateTime<Utc>,

    /// recent candle windows per timescale.
    /// each vec is ordered oldest-first, with the most recent candle last.
    /// length determined by the maximum lookback any active indicator needs.
    pub candles: HashMap<Timescale, Vec<Candle>>,

    /// pre-computed convenience fields
    pub spread: f64,
    pub session_vwap: f64,
    pub session_volume: f64,

    /// optional position context for meta-indicators
    pub position_context: Option<PositionContext>,

    /// session progress (0.0 = open, 1.0 = close).
    /// set by the execution loop based on current time vs session hours.
    pub session_progress: Option<f64>,

    /// set to true when entries are blocked (e.g., max concurrent positions reached).
    #[allow(dead_code)]
    pub entries_blocked: bool,

    /// total capital deployed across all tickers (for correlation-aware sizing).
    pub total_deployed_capital: Option<f64>,

    /// total initial capital across all tickers (for correlation-aware sizing).
    pub total_initial_capital: Option<f64>,

    /// index return (e.g., SPY daily return) for market breadth indicator.
    pub index_return: Option<f64>,

    /// cross-ticker correlation coefficient for cross-correlation indicator.
    pub cross_ticker_correlation: Option<f64>,
    /// cross-ticker context (index + peers) for the current bar. filled by the backtest
    /// from the bar cache (`--cross-index`); `None` in live until the feed populates it,
    /// so any window condition on it fails safe.
    pub cross: Option<CrossContext>,
}

/// what the index and the other traded names are doing on this bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrossContext {
    /// index close / index session open − 1
    pub index_session_ret: f64,
    /// index close / index close 5 bars ago − 1
    pub index_ret_5m: f64,
    /// index close / index close 15 bars ago − 1
    pub index_ret_15m: f64,
    /// fraction of the *other* traded tickers whose close is below their session open
    pub peers_red_frac: f64,
    /// mean session return of the other traded tickers
    pub peers_mean_session_ret: f64,
}
