use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::action::ActionPhase;
use crate::market::Timescale;

// ---- agent types ----

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

// ---- tool requests (from any timescale agent) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRequest {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub requesting_agent: AgentType,
    pub tool_category: ToolCategory,
    pub gap_description: GapDescription,
    pub evidence: Vec<EvidenceItem>,
    pub status: ToolRequestStatus,
    pub proposal_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    Indicator,
    Action,
    ScoringPipeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDescription {
    pub summary: String,
    pub condition: String,
    pub desired_behavior: String,
    pub existing_tool_limitations: String,
    pub interface_sketch: Option<InterfaceSketch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceSketch {
    pub name: String,
    pub timescales: Vec<Timescale>,
    pub expected_params: Vec<ParamSketch>,
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
    pub trade_ids: Vec<i64>,
    pub observation: String,
    pub estimated_impact: Option<String>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolRequestStatus {
    Pending,
    UnderReview,
    AcceptedForProposal,
    Declined,
    Superseded,
}

// ---- proposals (drafted by PM agent only) ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProposal {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub source_request_ids: Vec<i64>,
    pub title: String,
    pub status: ProposalStatus,
    pub analysis: ProposalAnalysis,
    pub spec: ToolSpec,
    pub pr: Option<PullRequestInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalAnalysis {
    pub rationale: String,
    pub timescale_impact: HashMap<Timescale, String>,
    pub risks: Vec<String>,
    pub interaction_with_existing: String,
    pub expected_improvement: ExpectedImprovement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedImprovement {
    pub target_metric: String,
    pub estimated_magnitude: String,
    pub affected_trade_count: Option<i32>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub category: ToolCategory,
    pub type_name: String,
    pub action_phase: Option<ActionPhase>,
    pub algorithm_description: String,
    pub required_inputs: Vec<String>,
    pub params: Vec<ParamSpec>,
    pub output_spec: OutputSpec,
    pub rust_implementation: String,
    pub test_cases: String,
    pub agent_documentation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    pub name: String,
    pub param_type: ParamType,
    pub description: String,
    pub default_value: serde_json::Value,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub tuned_by: Vec<AgentType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    Integer,
    Float,
    Boolean,
    String,
    Enum(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpec {
    pub normalization_method: String,
    pub metadata_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalStatus {
    Drafting,
    ReadyForPR,
    PRSubmitted,
    ChangesRequested,
    Merged,
    Rejected,
    Deployed,
}

// ---- github PR tracking ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub pr_number: i32,
    pub url: String,
    pub branch: String,
    pub files: Vec<PRFile>,
    pub state: PRState,
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
    pub file_path: Option<String>,
    pub line_number: Option<i32>,
}
