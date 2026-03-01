// ============================================================================
// ADAPTIVE MULTI-TIMESCALE TRADING SYSTEM
// Tool Belt & Knobs — Core Type Definitions
// ============================================================================
//
// Conceptual model:
//   TOOL BELT = two registries of pluggable modules
//     - Indicators: compute a score from market data (read-only)
//     - Actions: manage position lifecycle (entries, exits, sizing)
//   KNOBS = parameters on each tool, tuned by evolution agents
//   CONFIG = declarative manifest of active tools + their knob values
//
// The execution engine loads config → populates registries → runs the
// scoring pipeline and action evaluators every tick. Agents modify config
// between sessions; the engine hot-reloads.
// ============================================================================

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};


// ============================================================================
// MARKET DATA TYPES (inputs to indicators)
// ============================================================================

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

/// Snapshot of current market state across all timescales.
/// Rebuilt every tick from the price feed.
#[derive(Debug, Clone)]
pub struct MarketState {
    /// Current tick price
    pub last_price: f64,
    pub bid: f64,
    pub ask: f64,
    pub timestamp: DateTime<Utc>,

    /// Recent candle windows per timescale.
    /// Each vec is ordered oldest-first, with the most recent candle last.
    /// Length determined by the maximum lookback any active indicator needs.
    pub candles: HashMap<Timescale, Vec<Candle>>,

    /// Pre-computed convenience fields
    pub spread: f64,
    pub session_vwap: f64,
    pub session_volume: f64,
}


// ============================================================================
// INDICATOR SYSTEM (the "sensing" half of the tool belt)
// ============================================================================

/// Output from a single indicator computation.
#[derive(Debug, Clone, Serialize)]
pub struct IndicatorOutput {
    /// Normalized score: -1.0 (strong bearish) to +1.0 (strong bullish).
    /// Indicators should normalize their raw output to this range.
    pub score: f64,

    /// Raw computed value (e.g., RSI = 72.3, MACD histogram = 0.0012).
    /// Stored in trade snapshots for agent analysis.
    pub raw_value: f64,

    /// Optional secondary values (e.g., MACD has signal line + histogram).
    pub metadata: HashMap<String, f64>,
}

/// Trait every indicator must implement.
/// Indicators are pure functions: market state in, score out.
/// They must not hold mutable state between ticks (stateless computation).
pub trait Indicator: Send + Sync {
    /// Unique name used to reference this indicator in config and decision tree.
    fn name(&self) -> &str;

    /// Which timescale's candle data this indicator operates on.
    fn timescale(&self) -> Timescale;

    /// Minimum number of candles needed to produce a valid output.
    fn min_lookback(&self) -> usize;

    /// Compute indicator value from current market state.
    /// Returns None if insufficient data (e.g., not enough candles yet).
    fn compute(&self, market: &MarketState) -> Option<IndicatorOutput>;
}

/// Configuration for a single indicator instance.
/// This is what lives in the config file — the "knobs" for one indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorConfig {
    /// Indicator type name (maps to a factory function in the registry).
    /// e.g., "rsi", "macd", "vwap_distance", "volume_spike"
    pub indicator_type: String,

    /// Unique instance ID. Allows multiple instances of the same type
    /// with different params (e.g., "rsi_14" and "rsi_7").
    pub instance_id: String,

    /// Which timescale this instance is assigned to.
    pub timescale: Timescale,

    /// Is this indicator currently active in the scoring pipeline?
    pub enabled: bool,

    /// Weight in the timescale's score aggregation.
    /// Weights within a timescale are normalized to sum to 1.0.
    pub weight: f64,

    /// Arbitrary parameters specific to this indicator type.
    /// e.g., {"period": 14, "overbought": 70, "oversold": 30}
    /// Schema enforced by the indicator's factory, not by the config system.
    pub params: HashMap<String, serde_json::Value>,

    /// Metadata for evolution tracking (not used by execution engine)
    pub last_modified_by: Option<String>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub modification_reason: Option<String>,
}


// ============================================================================
// ACTION SYSTEM (the "doing" half of the tool belt)
// ============================================================================

/// What an action evaluator can tell the execution engine to do.
#[derive(Debug, Clone, Serialize)]
pub enum ActionSignal {
    /// Do nothing this tick.
    Hold,

    /// Enter a position.
    Enter {
        direction: TradeDirection,
        /// Suggested size as fraction of available capital.
        size_fraction: f64,
        /// Reason string for trade log.
        reason: String,
    },

    /// Exit current position (fully).
    Exit {
        reason: ExitReason,
    },

    /// Modify existing position parameters (e.g., tighten stop).
    ModifyStop {
        new_stop_price: f64,
    },

    /// Scale into/out of position.
    ScalePosition {
        /// Positive = add, negative = reduce. As fraction of current size.
        delta_fraction: f64,
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TradeDirection {
    Long,
    Short,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExitReason {
    TrailingStop,
    HardStop,
    TakeProfit,
    MaxHoldTimeout,
    SessionClose,
    FilterAlignment,
    ManualOverride,
    ConfigChange,
}

/// Current state of an open position (input to action evaluators).
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
    pub high_water_mark: f64,    // best price since entry (for trailing stops)
    pub low_water_mark: f64,     // worst price since entry
    pub hold_duration_ms: i64,
}

/// Trait every action module must implement.
/// Actions evaluate the current position + market state and may
/// produce signals that affect position management.
///
/// Unlike indicators, actions CAN be stateful within a position's
/// lifetime (e.g., a trailing stop tracks high water mark).
pub trait Action: Send + Sync {
    /// Unique name for this action type.
    fn name(&self) -> &str;

    /// What phase of the trade lifecycle does this action operate on?
    fn phase(&self) -> ActionPhase;

    /// Evaluate whether this action should fire.
    /// Called every tick while a position is open (for exit/monitor actions)
    /// or while flat (for entry actions).
    fn evaluate(
        &self,
        position: Option<&Position>,
        market: &MarketState,
        scores: &TimescaleScores,
    ) -> ActionSignal;
}

/// When in the trade lifecycle an action is relevant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPhase {
    /// Evaluated while flat, looking for entries.
    Entry,
    /// Evaluated while in a position, managing it.
    Monitor,
    /// Evaluated while in a position, looking for exits.
    Exit,
    /// Determines position size at entry time.
    Sizing,
}

/// Configuration for a single action instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionConfig {
    /// Action type name (maps to factory). e.g., "trailing_stop", "hard_stop",
    /// "score_threshold_entry", "max_hold_timer", "fixed_risk_sizing"
    pub action_type: String,

    /// Unique instance ID.
    pub instance_id: String,

    /// Lifecycle phase.
    pub phase: ActionPhase,

    /// Is this action currently active?
    pub enabled: bool,

    /// Priority within phase. Lower = evaluated first.
    /// If multiple exit actions fire on the same tick, highest priority wins.
    pub priority: i32,

    /// Arbitrary parameters.
    /// e.g., trailing stop: {"initial_offset_pct": 0.003, "tighten_threshold": 0.005}
    pub params: HashMap<String, serde_json::Value>,

    /// Evolution tracking
    pub last_modified_by: Option<String>,
    pub last_modified_at: Option<DateTime<Utc>>,
    pub modification_reason: Option<String>,
}


// ============================================================================
// SCORING PIPELINE (the decision tree connecting indicators → actions)
// ============================================================================

/// Aggregated scores per timescale, computed from all active indicators.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TimescaleScores {
    pub one_minute: Option<f64>,
    pub five_minute: Option<f64>,
    pub one_hour: Option<f64>,
    pub one_day: Option<f64>,
    pub one_month: Option<f64>,
    pub composite: f64,
}

/// How timescale scores are combined into the composite score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Weight per timescale in composite score calculation.
    /// These are the weights the PM agent tunes.
    pub timescale_weights: HashMap<Timescale, f64>,

    /// Composite score above this → eligible for entry.
    pub entry_threshold: f64,

    /// Composite score below this while in position → exit signal.
    pub exit_threshold: f64,

    /// Aggregation method.
    pub aggregation: AggregationMethod,

    /// Hard gates: timescales whose score must be positive (>0)
    /// regardless of composite score. Typically includes 1min/5min.
    pub hard_gate_timescales: Vec<Timescale>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationMethod {
    /// Simple weighted sum (default).
    WeightedSum,
    /// Weighted sum but floored at 0 if any hard gate fails.
    WeightedSumWithGates,
    /// Minimum of all timescale scores (conservative).
    MinScore,
}


// ============================================================================
// TOP-LEVEL CONFIG (the full manifest)
// ============================================================================

/// Complete strategy configuration. This is what gets serialized to
/// the database as the config blob. The execution engine deserializes
/// this on startup and on hot-reload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Schema version for forward compatibility.
    pub schema_version: String,

    /// Config metadata
    pub config_id: i64,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
    pub parent_config_id: Option<i64>,

    /// Instruments this config applies to.
    pub tickers: Vec<String>,

    /// All indicator instances (the sensing tool belt).
    pub indicators: Vec<IndicatorConfig>,

    /// All action instances (the doing tool belt).
    pub actions: Vec<ActionConfig>,

    /// Scoring pipeline configuration.
    pub scoring: ScoringConfig,

    /// Session-level rules.
    pub session: SessionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Don't open new positions after this time (HH:MM, exchange local).
    pub no_new_entries_after: String,

    /// Force close all positions by this time.
    pub force_exit_by: String,

    /// Skip the first N minutes of the session (opening volatility).
    pub avoid_first_minutes: u32,

    /// Maximum concurrent positions.
    pub max_concurrent_positions: u32,

    /// Maximum capital deployed as fraction of total.
    pub max_capital_deployed_pct: f64,
}


// ============================================================================
// INDICATOR & ACTION REGISTRIES (runtime, not serialized)
// ============================================================================

/// Registry that maps type names to factory functions.
/// On config load, the engine iterates config entries and calls
/// the appropriate factory to instantiate each tool.
///
/// This is the extension point for agents adding new tools:
/// a new indicator/action type means a new factory registered here.
/// The execution engine binary must be recompiled to add truly new
/// tool TYPES — but agents can freely add/remove/reconfigure INSTANCES
/// of existing types through config alone.
pub struct IndicatorRegistry {
    factories: HashMap<String, Box<dyn Fn(&IndicatorConfig) -> Box<dyn Indicator>>>,
}

pub struct ActionRegistry {
    factories: HashMap<String, Box<dyn Fn(&ActionConfig) -> Box<dyn Action>>>,
}

/// Runtime state: the loaded tool belt.
pub struct ToolBelt {
    /// Active indicator instances, keyed by instance_id.
    pub indicators: HashMap<String, Box<dyn Indicator>>,

    /// Active action instances, grouped by phase.
    pub entry_actions: Vec<Box<dyn Action>>,
    pub monitor_actions: Vec<Box<dyn Action>>,
    pub exit_actions: Vec<Box<dyn Action>>,
    pub sizing_actions: Vec<Box<dyn Action>>,
}


// ============================================================================
// EXECUTION LOOP PSEUDOCODE
// ============================================================================
//
// fn on_tick(market: &MarketState, belt: &ToolBelt, config: &StrategyConfig) {
//
//     // 1. Compute all indicator scores
//     let mut timescale_scores = TimescaleScores::default();
//     for (id, indicator) in &belt.indicators {
//         if let Some(output) = indicator.compute(market) {
//             let cfg = find_indicator_config(config, id);
//             accumulate_score(&mut timescale_scores, indicator.timescale(), output.score, cfg.weight);
//         }
//     }
//     compute_composite(&mut timescale_scores, &config.scoring);
//
//     // 2. If flat: evaluate entry actions
//     if no_open_position() {
//         if timescale_scores.composite >= config.scoring.entry_threshold
//             && hard_gates_pass(&timescale_scores, &config.scoring)
//         {
//             for action in &belt.entry_actions {
//                 match action.evaluate(None, market, &timescale_scores) {
//                     ActionSignal::Enter { direction, size_fraction, reason } => {
//                         let size = evaluate_sizing(&belt.sizing_actions, size_fraction, market);
//                         open_position(direction, size, &timescale_scores, reason);
//                         break;
//                     }
//                     _ => continue,
//                 }
//             }
//         }
//     }
//
//     // 3. If in position: evaluate exit actions (priority order)
//     if let Some(position) = get_open_position() {
//         // Monitor actions first (may modify stops, etc.)
//         for action in &belt.monitor_actions {
//             if let ActionSignal::ModifyStop { new_stop_price } =
//                 action.evaluate(Some(position), market, &timescale_scores)
//             {
//                 update_stop(new_stop_price);
//             }
//         }
//
//         // Exit actions in priority order — first one wins
//         for action in &belt.exit_actions {
//             if let ActionSignal::Exit { reason } =
//                 action.evaluate(Some(position), market, &timescale_scores)
//             {
//                 close_position(reason, &timescale_scores);
//                 break;
//             }
//         }
//     }
// }
// ============================================================================
