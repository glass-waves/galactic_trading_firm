use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::market::MarketState;
use crate::scoring::TimescaleScores;

/// what an action evaluator can tell the execution engine to do.
#[derive(Debug, Clone, Serialize)]
pub enum ActionSignal {
    /// do nothing this tick.
    Hold,

    /// enter a position.
    Enter {
        direction: TradeDirection,
        /// suggested size as fraction of available capital.
        size_fraction: f64,
        /// reason string for trade log.
        reason: String,
    },

    /// exit current position (fully).
    Exit { reason: ExitReason },

    /// modify existing position parameters (e.g., tighten stop).
    ModifyStop { new_stop_price: f64 },

    /// scale into/out of position.
    ScalePosition {
        /// positive = add, negative = reduce. as fraction of current size.
        delta_fraction: f64,
        reason: String,
    },

    /// reject entry — blocks all remaining entry actions this tick.
    /// used by entry reject gates to prevent low-quality entries.
    RejectEntry,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TradeDirection {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExitReason {
    TrailingStop,
    HardStop,
    TakeProfit,
    MaxHoldTimeout,
    SessionClose,
    FilterAlignment,
    ManualOverride,
    ConfigChange,
    ScoreExit,
    DailyLossLimit,
}

/// current state of an open position.
#[derive(Debug, Clone)]
pub struct Position {
    pub ticker: String,
    pub direction: TradeDirection,
    pub entry_price: f64,
    pub current_price: f64,
    pub size: f64,
    pub entry_time: DateTime<Utc>,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub high_water_mark: f64,
    pub low_water_mark: f64,
    pub hold_duration_ms: i64,
}

/// trait every action module must implement.
/// unlike indicators, actions CAN be stateful within a position's lifetime.
pub trait Action: Send + Sync {
    /// unique name for this action type.
    fn name(&self) -> &str;

    /// what phase of the trade lifecycle does this action operate on?
    fn phase(&self) -> ActionPhase;

    /// evaluate whether this action should fire.
    fn evaluate(
        &self,
        position: Option<&Position>,
        market: &MarketState,
        scores: &TimescaleScores,
    ) -> ActionSignal;
}

/// when in the trade lifecycle an action is relevant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPhase {
    Entry,
    Monitor,
    Exit,
    Sizing,
}

/// configuration for a single action instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionConfig {
    /// action type name (maps to factory).
    pub action_type: String,

    /// unique instance ID.
    pub instance_id: String,

    /// lifecycle phase.
    pub phase: ActionPhase,

    /// is this action currently active?
    pub enabled: bool,

    /// priority within phase. lower = evaluated first.
    pub priority: i32,

    /// arbitrary parameters.
    pub params: HashMap<String, serde_json::Value>,

    /// evolution tracking
    pub last_modified_by: Option<String>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub modification_reason: Option<String>,
}
