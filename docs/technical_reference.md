# Technical Reference — Consolidated Artifacts

**Adaptive Multi-Timescale Trading System | v0.3 | February 2026**

This document consolidates all Rust type definitions and SQL schema into a single downloadable reference. For the architecture diagram, see `system_architecture.mermaid`. For the project overview, see the project doc.

---

## Table of Contents

1. [Tool Belt & Knobs — Rust Types](#1-tool-belt--knobs)
2. [Proposal System — Rust Types](#2-proposal-system)
3. [Data Model — SQL Schema](#3-data-model)

---

## 1. Tool Belt & Knobs

Defines the indicator and action registries, scoring pipeline, and config structure for the Rust execution engine.

```rust
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
```

---

## 2. Proposal System

Defines the tool request/proposal lifecycle for the human-gated new tool pipeline.

```rust
// ============================================================================
// PROPOSAL SYSTEM — New Tool Lifecycle
// ============================================================================
//
// Flow:
//   1. Timescale agent identifies capability gap → writes ToolRequest
//   2. PM agent reviews requests → evaluates cross-timescale impact
//   3. PM drafts Proposal with implementation spec
//   4. PM submits PR to GitHub repo (code + docs + tests)
//   5. Human reviews, iterates, merges or rejects
//   6. On deploy, new tool type appears in registry
//   7. Agents can now configure instances of the new tool via normal config
//
// Key constraint: agents can never add tool TYPES to a running binary.
// They can only request new types through this human-gated process.
// ============================================================================

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};


// ============================================================================
// TOOL REQUESTS (from any timescale agent)
// ============================================================================

/// A timescale agent's identification of a missing capability.
/// Written during evolution cycles when the agent observes patterns
/// it cannot adequately handle with existing tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: i64,
    pub created_at: DateTime<Utc>,

    /// Which agent identified the gap.
    pub requesting_agent: AgentType,

    /// What kind of tool is needed?
    pub tool_category: ToolCategory,

    /// What capability is missing? Structured for PM consumption.
    pub gap_description: GapDescription,

    /// Evidence from recent trading that motivates this request.
    pub evidence: Vec<EvidenceItem>,

    /// Current status in the proposal pipeline.
    pub status: ToolRequestStatus,

    /// If the PM agent decided to act on this, link to proposal.
    pub proposal_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    /// New indicator type (sensing capability).
    Indicator,
    /// New action type (entry, exit, monitor, or sizing).
    Action,
    /// Modification to scoring pipeline logic.
    ScoringPipeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDescription {
    /// One-line summary of what's missing.
    /// e.g., "No way to detect volume-weighted momentum divergence at 1min scale"
    pub summary: String,

    /// What market condition or pattern triggers the need.
    /// e.g., "Price makes new high but volume-weighted momentum is declining"
    pub condition: String,

    /// What the agent would do differently if it had this tool.
    /// e.g., "Would reduce entry score when divergence detected, avoiding
    ///        entries into exhaustion moves"
    pub desired_behavior: String,

    /// Which existing tools are insufficient and why.
    /// e.g., "RSI catches some divergences but misses volume-weighted ones;
    ///        MACD is too slow at 1min timescale"
    pub existing_tool_limitations: String,

    /// Rough sketch of what the tool's interface would look like.
    /// Optional — the PM agent will formalize this if it drafts a proposal.
    pub interface_sketch: Option<InterfaceSketch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSketch {
    /// Suggested tool name.
    pub name: String,
    /// Which timescale(s) it would operate on.
    pub timescales: Vec<Timescale>,
    /// Expected parameters (knobs).
    pub expected_params: Vec<ParamSketch>,
    /// What the output would represent.
    pub output_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSketch {
    pub name: String,
    pub description: String,
    pub suggested_default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Reference to specific trades that demonstrate the gap.
    pub trade_ids: Vec<i64>,
    /// What happened in these trades.
    pub observation: String,
    /// Estimated P&L impact if the tool had existed.
    pub estimated_impact: Option<String>,
    /// Time period this evidence covers.
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolRequestStatus {
    /// Submitted by timescale agent, awaiting PM review.
    Pending,
    /// PM is evaluating this request.
    UnderReview,
    /// PM decided to draft a proposal.
    AcceptedForProposal,
    /// PM decided this isn't needed (with reason).
    Declined,
    /// Superseded by another request or existing tool update.
    Superseded,
}


// ============================================================================
// PROPOSALS (drafted by PM agent only)
// ============================================================================

/// A formal proposal for a new tool type, drafted by the PM agent
/// after evaluating one or more tool requests in cross-timescale context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProposal {
    pub id: i64,
    pub created_at: DateTime<Utc>,

    /// Which tool requests motivated this proposal.
    pub source_request_ids: Vec<i64>,

    /// Proposal metadata.
    pub title: String,
    pub status: ProposalStatus,

    /// PM's cross-timescale analysis.
    pub analysis: ProposalAnalysis,

    /// Implementation specification.
    pub spec: ToolSpec,

    /// If a PR was submitted, track it.
    pub pr: Option<PullRequestInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalAnalysis {
    /// PM's reasoning for why this tool should exist.
    pub rationale: String,

    /// Expected impact on each timescale.
    pub timescale_impact: HashMap<Timescale, String>,

    /// Potential negative interactions or risks.
    pub risks: Vec<String>,

    /// How this interacts with existing tools.
    pub interaction_with_existing: String,

    /// Expected improvement metrics.
    pub expected_improvement: ExpectedImprovement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedImprovement {
    /// What metric would improve and by roughly how much.
    pub target_metric: String,
    pub estimated_magnitude: String,
    /// How many trades from the evidence period would have been affected.
    pub affected_trade_count: Option<i32>,
    /// Confidence level in this estimate.
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Tool type (indicator or action).
    pub category: ToolCategory,

    /// Proposed type name for the registry.
    pub type_name: String,

    /// For actions: which phase.
    pub action_phase: Option<ActionPhase>,

    /// Detailed description of the computation / logic.
    pub algorithm_description: String,

    /// Input requirements.
    pub required_inputs: Vec<String>,

    /// Parameter definitions with types, ranges, and defaults.
    pub params: Vec<ParamSpec>,

    /// Output description.
    pub output_spec: OutputSpec,

    /// Proposed Rust implementation.
    /// The PM agent generates this; human reviews in the PR.
    pub rust_implementation: String,

    /// Proposed unit tests.
    pub test_cases: String,

    /// Documentation for future agents.
    pub agent_documentation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    pub name: String,
    pub param_type: ParamType,
    pub description: String,
    pub default_value: serde_json::Value,
    /// Valid range for numeric params.
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    /// Which agent(s) should tune this param.
    pub tuned_by: Vec<AgentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    Integer,
    Float,
    Boolean,
    String,
    /// One of a fixed set of string options.
    Enum(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpec {
    /// How the raw output maps to the normalized -1.0 to +1.0 score.
    pub normalization_method: String,
    /// What metadata fields are included.
    pub metadata_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    /// PM is still drafting.
    Drafting,
    /// PM has finalized, ready to submit PR.
    ReadyForPR,
    /// PR has been submitted to GitHub.
    PRSubmitted,
    /// Human requested changes on the PR.
    ChangesRequested,
    /// PR merged, tool available after next deploy.
    Merged,
    /// PR rejected by human.
    Rejected,
    /// Tool deployed and available in registry.
    Deployed,
}


// ============================================================================
// GITHUB PR TRACKING
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    /// GitHub PR number.
    pub pr_number: i32,
    /// GitHub PR URL.
    pub url: String,
    /// Branch name.
    pub branch: String,
    /// Files included in the PR.
    pub files: Vec<PRFile>,
    /// Current PR state.
    pub state: PRState,
    /// Human review comments (fetched from GitHub).
    pub review_comments: Vec<ReviewComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRFile {
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PRState {
    Open,
    ChangesRequested,
    Approved,
    Merged,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    /// If the comment requests specific changes, the PM agent
    /// can parse this and iterate on the implementation.
    pub file_path: Option<String>,
    pub line_number: Option<i32>,
}


// ============================================================================
// SQL TABLES (additional tables for proposal system)
// ============================================================================
//
// CREATE TABLE tool_requests (
//     id                  BIGSERIAL PRIMARY KEY,
//     created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
//     requesting_agent    agent_type NOT NULL,
//     tool_category       VARCHAR(20) NOT NULL,
//     status              VARCHAR(30) NOT NULL DEFAULT 'pending',
//     gap_summary         TEXT NOT NULL,
//     gap_condition       TEXT NOT NULL,
//     gap_desired_behavior TEXT NOT NULL,
//     gap_existing_limitations TEXT NOT NULL,
//     interface_sketch    JSONB,
//     evidence            JSONB NOT NULL DEFAULT '[]',
//     proposal_id         BIGINT REFERENCES tool_proposals(id),
//     decline_reason      TEXT
// );
//
// CREATE TABLE tool_proposals (
//     id                  BIGSERIAL PRIMARY KEY,
//     created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
//     title               TEXT NOT NULL,
//     status              VARCHAR(30) NOT NULL DEFAULT 'drafting',
//     source_request_ids  BIGINT[] NOT NULL,
//     analysis            JSONB NOT NULL,
//     spec                JSONB NOT NULL,
//     pr_number           INTEGER,
//     pr_url              TEXT,
//     pr_branch           TEXT,
//     pr_state            VARCHAR(20),
//     pr_files            JSONB,
//     review_comments     JSONB DEFAULT '[]'
// );
//
// CREATE INDEX idx_requests_status ON tool_requests(status);
// CREATE INDEX idx_requests_agent ON tool_requests(requesting_agent);
// CREATE INDEX idx_proposals_status ON tool_proposals(status);
// ============================================================================


// Re-exports for convenience (these are defined in tool_belt_types.rs)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgentType {
    Agent1Min,
    Agent5Min,
    AgentHourly,
    AgentDaily,
    AgentMonthly,
    AgentPM,
    Orchestrator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Timescale {
    OneMinute,
    FiveMinute,
    OneHour,
    OneDay,
    OneMonth,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionPhase {
    Entry,
    Monitor,
    Exit,
    Sizing,
}
```

---

## 3. Data Model

Complete SQL schema including config versioning, trade events, agent memos (with observation/recommendation types), evolution cycles (full PM + check-in), config changelog, daily budget tracking, and analysis views.

```sql
-- ============================================================================
-- ADAPTIVE MULTI-TIMESCALE TRADING SYSTEM — DATA MODEL
-- ============================================================================
-- Version: 0.3
-- Database: Postgres (SQLite-compatible with minor type adjustments)
--
-- Design principles:
--   - Config is immutable/append-only (never update in place)
--   - Trade events capture full decision context at time of trade
--   - Indicator values at trade time are cached; all other price data
--     is reconstructed from historical feed on demand
--   - Agent memos are persisted for longitudinal analysis
--   - Two-tier evolution: full PM cycles + lightweight check-ins
--   - All timestamps are UTC microsecond precision
-- ============================================================================


-- ============================================================================
-- ENUMS
-- ============================================================================

CREATE TYPE timescale AS ENUM ('1min', '5min', '1hour', '1day', '1month');

CREATE TYPE trade_direction AS ENUM ('long', 'short');

CREATE TYPE exit_reason AS ENUM (
    'filter_alignment',   -- exit filters aligned normally
    'trailing_stop',      -- trailing stop triggered
    'hard_stop',          -- hard stop loss hit
    'max_hold_timeout',   -- exceeded maximum hold duration
    'take_profit',        -- take profit target reached
    'session_close',      -- market close forced exit
    'manual_override',    -- human intervention
    'config_change'       -- config reload forced position close
);

CREATE TYPE agent_type AS ENUM (
    'agent_1min',
    'agent_5min',
    'agent_hourly',
    'agent_daily',
    'agent_monthly',
    'agent_pm',
    'orchestrator'
);

CREATE TYPE memo_type AS ENUM (
    'observation',         -- lightweight check-in: read-only, no config authority
    'recommendation'       -- full PM cycle: may propose config changes
);

CREATE TYPE cycle_type AS ENUM (
    'full_pm',             -- full PM-orchestrated evolution cycle (1-3x daily)
    'checkin'              -- lightweight observation-only cycle (more frequent)
);

CREATE TYPE mutation_status AS ENUM (
    'proposed',           -- agent proposed, not yet backtested
    'backtesting',        -- currently being validated
    'validated',          -- passed backtest, awaiting promotion
    'promoted',           -- live in execution engine
    'rejected',           -- failed backtest validation
    'rolled_back',        -- was promoted but performance degraded
    'superseded'          -- replaced by a newer version
);


-- ============================================================================
-- STRATEGY CONFIGURATION (immutable append-only)
-- ============================================================================
-- Every config change produces a new row. The execution engine reads
-- the latest 'promoted' version. Rollback = promote an older version.
-- ============================================================================

CREATE TABLE config_versions (
    id                  BIGSERIAL PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    status              mutation_status NOT NULL DEFAULT 'proposed',
    promoted_at         TIMESTAMPTZ,
    rolled_back_at      TIMESTAMPTZ,

    -- Who created this version and why
    created_by          agent_type NOT NULL,
    parent_version_id   BIGINT REFERENCES config_versions(id),
    mutation_reason     TEXT NOT NULL,          -- freeform: why the agent made this change

    -- The full config snapshot as JSON. Schema will evolve.
    -- Contains all filter params, scoring weights, tooling params.
    config_blob         JSONB NOT NULL,

    -- Backtest results for this config (populated after validation)
    backtest_sharpe     DOUBLE PRECISION,
    backtest_win_rate   DOUBLE PRECISION,
    backtest_total_trades INTEGER,
    backtest_period_start TIMESTAMPTZ,
    backtest_period_end   TIMESTAMPTZ,
    backtest_results    JSONB                   -- detailed backtest output
);

CREATE INDEX idx_config_status ON config_versions(status);
CREATE INDEX idx_config_created_by ON config_versions(created_by);
CREATE INDEX idx_config_created_at ON config_versions(created_at DESC);

-- View: the currently active config
CREATE VIEW active_config AS
SELECT * FROM config_versions
WHERE status = 'promoted'
ORDER BY promoted_at DESC
LIMIT 1;


-- ============================================================================
-- TRADE EVENTS
-- ============================================================================
-- One row per completed trade (entry + exit).
-- Captures the full decision context at both entry and exit.
-- ============================================================================

CREATE TABLE trades (
    id                  BIGSERIAL PRIMARY KEY,
    ticker              VARCHAR(10) NOT NULL,
    direction           trade_direction NOT NULL,

    -- Timestamps (microsecond precision)
    entry_signal_at     TIMESTAMPTZ NOT NULL,   -- when filters aligned for entry
    entry_fill_at       TIMESTAMPTZ NOT NULL,   -- when broker confirmed fill
    exit_signal_at      TIMESTAMPTZ NOT NULL,   -- when exit condition triggered
    exit_fill_at        TIMESTAMPTZ NOT NULL,   -- when broker confirmed exit fill

    -- Prices
    entry_price         DOUBLE PRECISION NOT NULL,
    exit_price          DOUBLE PRECISION NOT NULL,
    position_size       DOUBLE PRECISION NOT NULL,  -- number of shares/contracts

    -- Outcome
    pnl_dollars         DOUBLE PRECISION NOT NULL,
    pnl_percent         DOUBLE PRECISION NOT NULL,
    commission          DOUBLE PRECISION NOT NULL DEFAULT 0,
    slippage_entry      DOUBLE PRECISION,       -- signal price vs fill price
    slippage_exit       DOUBLE PRECISION,
    hold_duration_ms    BIGINT NOT NULL,         -- exit_fill_at - entry_fill_at

    -- Why did we exit?
    exit_reason         exit_reason NOT NULL,

    -- Config context
    config_version_id   BIGINT NOT NULL REFERENCES config_versions(id),

    -- Per-timescale scores at ENTRY (what the decision tree saw)
    entry_score_1min    DOUBLE PRECISION,
    entry_score_5min    DOUBLE PRECISION,
    entry_score_hourly  DOUBLE PRECISION,
    entry_score_daily   DOUBLE PRECISION,
    entry_score_monthly DOUBLE PRECISION,
    entry_score_composite DOUBLE PRECISION NOT NULL,  -- final aggregated score

    -- Per-timescale scores at EXIT
    exit_score_1min     DOUBLE PRECISION,
    exit_score_5min     DOUBLE PRECISION,
    exit_score_hourly   DOUBLE PRECISION,
    exit_score_daily    DOUBLE PRECISION,
    exit_score_monthly  DOUBLE PRECISION,
    exit_score_composite DOUBLE PRECISION,

    -- Tooling state at entry
    trailing_stop_initial DOUBLE PRECISION,     -- where the trailing stop was set
    take_profit_target    DOUBLE PRECISION,
    max_hold_timeout_ms   BIGINT,

    -- Paper vs live
    is_paper            BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_trades_ticker ON trades(ticker);
CREATE INDEX idx_trades_entry_at ON trades(entry_fill_at DESC);
CREATE INDEX idx_trades_config ON trades(config_version_id);
CREATE INDEX idx_trades_exit_reason ON trades(exit_reason);
CREATE INDEX idx_trades_direction ON trades(direction);

-- Composite index for common agent queries:
-- "show me recent trades where hourly was weak but 5min was strong"
CREATE INDEX idx_trades_score_analysis ON trades(
    entry_score_hourly, entry_score_5min, pnl_percent
);


-- ============================================================================
-- INDICATOR SNAPSHOTS AT TRADE TIME
-- ============================================================================
-- Cached indicator values at entry and exit for each timescale.
-- Avoids expensive recomputation when agents analyze trades.
-- One row per trade per timescale per event (entry/exit).
-- ============================================================================

CREATE TYPE trade_event_type AS ENUM ('entry', 'exit');

CREATE TABLE trade_indicator_snapshots (
    id                  BIGSERIAL PRIMARY KEY,
    trade_id            BIGINT NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
    event_type          trade_event_type NOT NULL,
    timescale           timescale NOT NULL,
    snapshot_at         TIMESTAMPTZ NOT NULL,

    -- Indicator values as JSON since they vary by timescale.
    -- 1min/5min: RSI, MACD, stochastic, bollinger z-score, RoC, etc.
    -- Hourly: VWAP distance, volume ratio, trend slope, etc.
    -- Daily: gap size, daily range position, sector relative strength, etc.
    -- Monthly: VIX level, regime score, broad index trend, etc.
    indicators          JSONB NOT NULL,

    UNIQUE (trade_id, event_type, timescale)
);

CREATE INDEX idx_snapshots_trade ON trade_indicator_snapshots(trade_id);
CREATE INDEX idx_snapshots_timescale ON trade_indicator_snapshots(timescale);


-- ============================================================================
-- AGENT MEMOS
-- ============================================================================
-- Structured outputs from each timescale agent, consumed by the PM agent.
-- Persisted for longitudinal analysis ("has the 5min agent been reporting
-- low confidence for weeks? did performance actually degrade?").
-- ============================================================================

CREATE TABLE agent_memos (
    id                  BIGSERIAL PRIMARY KEY,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    agent               agent_type NOT NULL,
    evolution_cycle_id  BIGINT NOT NULL,        -- groups memos from the same run
    memo_type           memo_type NOT NULL DEFAULT 'recommendation',

    -- Structured fields (PM agent's primary input)
    confidence_score    DOUBLE PRECISION,        -- 0-1, agent's self-assessed confidence
    volatility_regime   VARCHAR(20),             -- 'low', 'normal', 'high', 'extreme'
    directional_bias    VARCHAR(20),             -- 'strong_long', 'lean_long', 'neutral', 'lean_short', 'strong_short'
    signal_quality      VARCHAR(20),             -- 'strong', 'moderate', 'weak', 'conflicting'

    -- Pattern flags (boolean signals the PM agent can key off)
    flags               JSONB NOT NULL DEFAULT '{}',
    -- Examples:
    -- {"divergence_detected": true, "volume_anomaly": true, "level_rejection": false}

    -- Freeform reasoning (goes to RAG store, PM reads for context)
    reasoning           TEXT NOT NULL,

    -- What changes did this agent propose, if any?
    proposed_config_version_id BIGINT REFERENCES config_versions(id),

    -- Performance summary the agent was looking at
    review_period_start TIMESTAMPTZ,
    review_period_end   TIMESTAMPTZ,
    trades_reviewed     INTEGER,
    period_win_rate     DOUBLE PRECISION,
    period_sharpe       DOUBLE PRECISION,
    period_pnl          DOUBLE PRECISION
);

CREATE INDEX idx_memos_agent ON agent_memos(agent);
CREATE INDEX idx_memos_cycle ON agent_memos(evolution_cycle_id);
CREATE INDEX idx_memos_created ON agent_memos(created_at DESC);


-- ============================================================================
-- EVOLUTION CYCLES
-- ============================================================================
-- One row per evolution run. Two types:
--   - full_pm: complete PM-orchestrated cycle (1-3x daily, Sonnet 4.5)
--     All agents run, PM has config authority, changelog written.
--   - checkin: lightweight observation cycle (more frequent, Haiku 4.5)
--     Only fast agents (1min, 5min, optionally hourly) run.
--     They write observation memos but have ZERO config authority.
--     Builds richer evidence base for the next full PM cycle.
-- ============================================================================

CREATE TABLE evolution_cycles (
    id                  BIGSERIAL PRIMARY KEY,
    started_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at        TIMESTAMPTZ,
    trading_date        DATE NOT NULL,           -- which trading day this cycle reviews

    -- Cycle type and model
    cycle_type          cycle_type NOT NULL DEFAULT 'full_pm',
    model_used          VARCHAR(50),             -- e.g., 'claude-sonnet-4-5', 'claude-haiku-4-5'

    -- Which agents ran and their status
    agents_triggered    agent_type[] NOT NULL,
    agents_completed    agent_type[] NOT NULL DEFAULT '{}',

    -- Outcome (only populated for full_pm cycles)
    configs_proposed    INTEGER NOT NULL DEFAULT 0,
    configs_promoted    INTEGER NOT NULL DEFAULT 0,
    configs_rejected    INTEGER NOT NULL DEFAULT 0,

    -- Aggregate trading day stats
    day_total_trades    INTEGER,
    day_total_pnl       DOUBLE PRECISION,
    day_win_rate        DOUBLE PRECISION,
    day_sharpe          DOUBLE PRECISION,

    -- Cost tracking
    input_tokens_used   BIGINT DEFAULT 0,
    output_tokens_used  BIGINT DEFAULT 0,
    estimated_cost_usd  DOUBLE PRECISION DEFAULT 0
);

CREATE INDEX idx_cycles_date ON evolution_cycles(trading_date DESC);


-- ============================================================================
-- CONFIG CHANGELOG
-- ============================================================================
-- Detailed, agent-readable log of every config change.
-- Each row describes ONE atomic change (a single knob turned, a tool
-- enabled/disabled, a weight adjusted). A config version that changes
-- 3 params produces 3 changelog rows.
--
-- Timescale agents read this at the start of every evolution cycle
-- alongside trade data so they can attribute performance shifts to
-- config changes vs. market conditions.
-- ============================================================================

CREATE TYPE change_category AS ENUM (
    'knob_tuned',            -- parameter value changed on existing tool
    'tool_enabled',          -- existing tool instance activated
    'tool_disabled',         -- existing tool instance deactivated
    'tool_instance_added',   -- new instance of existing tool type added
    'tool_instance_removed', -- instance removed from config
    'weight_adjusted',       -- timescale weight or tool weight changed
    'threshold_adjusted',    -- entry/exit threshold changed
    'session_rule_changed',  -- session config changed
    'scoring_changed',       -- aggregation method or hard gates changed
    'new_tool_type_deployed' -- new tool type available after code deploy
);

CREATE TABLE config_changelog (
    id                  BIGSERIAL PRIMARY KEY,
    config_version_id   BIGINT NOT NULL REFERENCES config_versions(id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    changed_by          agent_type NOT NULL,

    -- What changed
    change_category     change_category NOT NULL,

    -- Which tool/component was affected
    target_timescale    timescale,              -- NULL if cross-cutting (PM tooling)
    target_tool_id      VARCHAR(100),           -- instance_id of affected tool
    target_tool_type    VARCHAR(100),           -- e.g., "rsi", "trailing_stop"
    target_param        VARCHAR(100),           -- specific param name, if knob_tuned

    -- Values
    old_value           JSONB,                  -- previous value (NULL for new additions)
    new_value           JSONB NOT NULL,         -- new value

    -- Context: why this change was made
    reason              TEXT NOT NULL,           -- agent's reasoning
    evidence_trade_ids  BIGINT[],               -- trades that motivated this change
    evidence_period     TSTZRANGE,              -- time window analyzed

    -- Link to the agent memo that produced this change
    source_memo_id      BIGINT REFERENCES agent_memos(id),

    -- For rollback tracking: if this change reverts a previous change
    reverts_changelog_id BIGINT REFERENCES config_changelog(id)
);

CREATE INDEX idx_changelog_config ON config_changelog(config_version_id);
CREATE INDEX idx_changelog_timescale ON config_changelog(target_timescale);
CREATE INDEX idx_changelog_tool ON config_changelog(target_tool_id);
CREATE INDEX idx_changelog_category ON config_changelog(change_category);
CREATE INDEX idx_changelog_created ON config_changelog(created_at DESC);
CREATE INDEX idx_changelog_agent ON config_changelog(changed_by);

-- View: recent changes relevant to a specific timescale
CREATE VIEW changelog_by_timescale AS
SELECT
    cl.*,
    cv.promoted_at,
    cv.status AS config_status
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status IN ('promoted', 'rolled_back')
ORDER BY cl.created_at DESC;

-- View: changes since a given config version (agent catch-up)
CREATE VIEW changelog_since AS
SELECT
    cl.*,
    cv.promoted_at
FROM config_changelog cl
JOIN config_versions cv ON cl.config_version_id = cv.id
WHERE cv.status = 'promoted'
ORDER BY cv.promoted_at ASC, cl.id ASC;


-- ============================================================================
-- CONFIG BLOB SCHEMA REFERENCE
-- ============================================================================
-- The config_blob JSONB in config_versions follows this structure.
-- Documented here as reference; enforced in application code, not DB.
--
-- {
--   "version": "0.1",
--   "last_modified_by": "agent_pm",
--
--   "filters": {
--     "1min": {
--       "enabled": true,
--       "weight": 0.15,
--       "params": {
--         "rsi_period": 14,
--         "rsi_overbought": 72,
--         "rsi_oversold": 28,
--         "volume_spike_threshold": 2.5,
--         "tape_momentum_window": 30
--       }
--     },
--     "5min": {
--       "enabled": true,
--       "weight": 0.25,
--       "params": {
--         "rsi_period": 14,
--         "rsi_overbought": 70,
--         "rsi_oversold": 30,
--         "macd_fast": 12,
--         "macd_slow": 26,
--         "macd_signal": 9,
--         "bollinger_period": 20,
--         "bollinger_std": 2.0
--       }
--     },
--     "1hour": {
--       "enabled": true,
--       "weight": 0.25,
--       "params": {
--         "vwap_distance_threshold": 0.005,
--         "volume_acceleration_window": 5,
--         "trend_slope_period": 10
--       }
--     },
--     "1day": {
--       "enabled": true,
--       "weight": 0.20,
--       "params": {
--         "gap_significance_threshold": 0.005,
--         "daily_range_lookback": 20,
--         "sector_strength_benchmark": "SPY"
--       }
--     },
--     "1month": {
--       "enabled": true,
--       "weight": 0.15,
--       "params": {
--         "vix_regime_thresholds": [15, 20, 30],
--         "trend_ma_period": 50,
--         "regime_lookback_days": 60
--       }
--     }
--   },
--
--   "scoring": {
--     "entry_threshold": 0.65,
--     "exit_threshold": -0.30,
--     "aggregation_method": "weighted_sum"
--   },
--
--   "tooling": {
--     "trailing_stop": {
--       "enabled": true,
--       "initial_offset_pct": 0.003,
--       "tighten_after_profit_pct": 0.005,
--       "tightened_offset_pct": 0.002
--     },
--     "hard_stop": {
--       "enabled": true,
--       "max_loss_pct": 0.01
--     },
--     "take_profit": {
--       "enabled": false,
--       "target_pct": 0.008
--     },
--     "max_hold": {
--       "timeout_minutes": 30,
--       "extend_if_profitable": true,
--       "extended_timeout_minutes": 60
--     },
--     "position_sizing": {
--       "method": "fixed_risk",
--       "risk_per_trade_pct": 0.01,
--       "max_position_pct": 0.05
--     },
--     "session_rules": {
--       "no_new_entries_after": "15:30",
--       "force_exit_by": "15:55",
--       "avoid_first_minutes": 5
--     }
--   },
--
--   "tickers": ["SPY", "QQQ", "AAPL", "NVDA", "MSFT"]
-- }
-- ============================================================================


-- ============================================================================
-- DAILY BUDGET TRACKING
-- ============================================================================
-- The Python orchestrator writes a row per day to enforce spend caps.
-- The orchestrator checks this before starting any new cycle.
-- ============================================================================

CREATE TABLE daily_budget (
    trading_date        DATE PRIMARY KEY,
    total_input_tokens  BIGINT NOT NULL DEFAULT 0,
    total_output_tokens BIGINT NOT NULL DEFAULT 0,
    total_cost_usd      DOUBLE PRECISION NOT NULL DEFAULT 0,
    full_pm_cycles      INTEGER NOT NULL DEFAULT 0,
    checkin_cycles      INTEGER NOT NULL DEFAULT 0,
    budget_limit_usd    DOUBLE PRECISION NOT NULL DEFAULT 5.00,   -- configurable daily cap
    budget_exhausted    BOOLEAN NOT NULL DEFAULT FALSE
);


-- ============================================================================
-- USEFUL VIEWS FOR AGENT QUERIES
-- ============================================================================

-- Daily performance summary (used by most agents)
CREATE VIEW daily_performance AS
SELECT
    date_trunc('day', entry_fill_at) AS trading_day,
    ticker,
    config_version_id,
    COUNT(*) AS total_trades,
    SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END) AS winning_trades,
    ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
    ROUND(SUM(pnl_dollars)::numeric, 2) AS total_pnl,
    ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
        / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
    ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms,
    ROUND(AVG(slippage_entry)::numeric, 6) AS avg_slippage_entry
FROM trades
WHERE NOT is_paper OR TRUE  -- include paper trades for now
GROUP BY date_trunc('day', entry_fill_at), ticker, config_version_id;

-- Performance bucketed by exit reason (used by PM agent)
CREATE VIEW performance_by_exit_reason AS
SELECT
    exit_reason,
    COUNT(*) AS total_trades,
    ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
    ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
        / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate,
    ROUND(AVG(hold_duration_ms)::numeric, 0) AS avg_hold_ms
FROM trades
GROUP BY exit_reason;

-- Score interaction analysis (used by PM agent to understand
-- how timescale combinations affect outcomes)
CREATE VIEW score_interaction_analysis AS
SELECT
    CASE
        WHEN entry_score_1min > 0.6 THEN 'strong'
        WHEN entry_score_1min > 0.3 THEN 'moderate'
        ELSE 'weak'
    END AS score_1min_bucket,
    CASE
        WHEN entry_score_5min > 0.6 THEN 'strong'
        WHEN entry_score_5min > 0.3 THEN 'moderate'
        ELSE 'weak'
    END AS score_5min_bucket,
    CASE
        WHEN entry_score_hourly > 0.6 THEN 'strong'
        WHEN entry_score_hourly > 0.3 THEN 'moderate'
        ELSE 'weak'
    END AS score_hourly_bucket,
    COUNT(*) AS total_trades,
    ROUND(AVG(pnl_percent)::numeric, 6) AS avg_pnl_pct,
    ROUND((SUM(CASE WHEN pnl_dollars > 0 THEN 1 ELSE 0 END)::float
        / NULLIF(COUNT(*), 0))::numeric, 4) AS win_rate
FROM trades
GROUP BY score_1min_bucket, score_5min_bucket, score_hourly_bucket
HAVING COUNT(*) >= 5;

-- Recent agent memo history (used to check for persistent signals)
CREATE VIEW recent_agent_signals AS
SELECT
    agent,
    memo_type,
    created_at,
    confidence_score,
    directional_bias,
    signal_quality,
    flags,
    trades_reviewed,
    period_win_rate,
    period_sharpe
FROM agent_memos
WHERE created_at > now() - interval '30 days'
ORDER BY agent, created_at DESC;

-- Check-in memos since last full PM cycle (PM reads these for context)
CREATE VIEW checkin_memos_since_last_pm AS
SELECT am.*
FROM agent_memos am
WHERE am.memo_type = 'observation'
  AND am.created_at > (
      SELECT MAX(ec.completed_at)
      FROM evolution_cycles ec
      WHERE ec.cycle_type = 'full_pm'
        AND ec.completed_at IS NOT NULL
  )
ORDER BY am.created_at ASC;

-- Daily cost summary
CREATE VIEW daily_cost_summary AS
SELECT
    trading_date,
    total_cost_usd,
    full_pm_cycles,
    checkin_cycles,
    budget_limit_usd,
    budget_exhausted,
    ROUND((total_cost_usd / NULLIF(budget_limit_usd, 0) * 100)::numeric, 1) AS budget_pct_used
FROM daily_budget
ORDER BY trading_date DESC;
```
