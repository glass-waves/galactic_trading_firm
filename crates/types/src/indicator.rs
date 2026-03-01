use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::market::{MarketState, Timescale};

/// output from a single indicator computation.
#[derive(Debug, Clone, Serialize)]
pub struct IndicatorOutput {
    /// normalized score: -1.0 (strong bearish) to +1.0 (strong bullish).
    pub score: f64,

    /// raw computed value (e.g., RSI = 72.3, MACD histogram = 0.0012).
    pub raw_value: f64,

    /// optional secondary values (e.g., MACD has signal line + histogram).
    pub metadata: HashMap<String, f64>,
}

/// trait every indicator must implement.
/// indicators are pure functions: market state in, score out.
/// they must not hold mutable state between ticks (stateless computation).
pub trait Indicator: Send + Sync {
    /// unique name used to reference this indicator in config and decision tree.
    fn name(&self) -> &str;

    /// which timescale's candle data this indicator operates on.
    fn timescale(&self) -> Timescale;

    /// minimum number of candles needed to produce a valid output.
    fn min_lookback(&self) -> usize;

    /// compute indicator value from current market state.
    /// returns None if insufficient data (e.g., not enough candles yet).
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput>;
}

/// configuration for a single indicator instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorConfig {
    /// indicator type name (maps to a factory function in the registry).
    pub indicator_type: String,

    /// unique instance ID.
    pub instance_id: String,

    /// which timescale this instance is assigned to.
    pub timescale: Timescale,

    /// is this indicator currently active in the scoring pipeline?
    pub enabled: bool,

    /// weight in the timescale's score aggregation.
    pub weight: f64,

    /// arbitrary parameters specific to this indicator type.
    pub params: HashMap<String, serde_json::Value>,

    /// metadata for evolution tracking
    pub last_modified_by: Option<String>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub modification_reason: Option<String>,
}
