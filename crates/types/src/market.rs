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
}
