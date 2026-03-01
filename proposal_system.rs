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
