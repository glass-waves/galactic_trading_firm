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
}
