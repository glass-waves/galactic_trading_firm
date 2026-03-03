/**
 * types matching the postgres schema.
 *
 * these types serve as the data layer between the database and the agent code.
 * they match the tables defined in docs/data_model.sql.
 */

// --- enums as string unions ---

export type AgentType =
  | "agent_analysis"
  | "agent_pm"
  | "orchestrator";

export type MemoType = "observation" | "analysis" | "recommendation";

export type CycleType = "full_pm" | "checkin";

export type VolatilityRegime = "low" | "normal" | "high" | "extreme";

export type DirectionalBias =
  | "strong_long"
  | "lean_long"
  | "neutral"
  | "lean_short"
  | "strong_short";

export type SignalQuality = "strong" | "moderate" | "weak" | "conflicting";

// --- structured suggestion ---

export interface Suggestion {
  target_tool_id: string;
  param: string | null;
  current_value: unknown;
  proposed_value: unknown;
  confidence: number;
  evidence_summary: string;
}

// --- agent memo ---

export interface AgentMemo {
  agent: AgentType;
  evolution_cycle_id: number;
  memo_type: MemoType;
  confidence_score: number | null;
  volatility_regime: VolatilityRegime | null;
  directional_bias: DirectionalBias | null;
  signal_quality: SignalQuality | null;
  flags: Record<string, unknown>;
  reasoning: string;
  proposed_config_version_id: number | null;
  review_period_start: string | null;
  review_period_end: string | null;
  trades_reviewed: number | null;
  period_win_rate: number | null;
  period_sharpe: number | null;
  period_pnl: number | null;
  suggestions: Suggestion[];
}

export function createAgentMemo(
  partial: Pick<AgentMemo, "agent" | "evolution_cycle_id"> &
    Partial<AgentMemo>,
): AgentMemo {
  return {
    memo_type: "observation",
    confidence_score: null,
    volatility_regime: null,
    directional_bias: null,
    signal_quality: null,
    flags: {},
    reasoning: "",
    proposed_config_version_id: null,
    review_period_start: null,
    review_period_end: null,
    trades_reviewed: null,
    period_win_rate: null,
    period_sharpe: null,
    period_pnl: null,
    suggestions: [],
    ...partial,
  };
}

// --- evolution cycle ---

export interface EvolutionCycle {
  trading_date: string;
  cycle_type: CycleType;
  model_used: string;
  agents_triggered: AgentType[];
  agents_completed: AgentType[];
  configs_proposed: number;
  configs_promoted: number;
  configs_rejected: number;
  day_total_trades: number | null;
  day_total_pnl: number | null;
  day_win_rate: number | null;
  day_sharpe: number | null;
  input_tokens_used: number;
  output_tokens_used: number;
  estimated_cost_usd: number;
}

export function createEvolutionCycle(
  partial: Pick<EvolutionCycle, "trading_date"> & Partial<EvolutionCycle>,
): EvolutionCycle {
  return {
    cycle_type: "checkin",
    model_used: "claude-sonnet-4-6",
    agents_triggered: [],
    agents_completed: [],
    configs_proposed: 0,
    configs_promoted: 0,
    configs_rejected: 0,
    day_total_trades: null,
    day_total_pnl: null,
    day_win_rate: null,
    day_sharpe: null,
    input_tokens_used: 0,
    output_tokens_used: 0,
    estimated_cost_usd: 0.0,
    ...partial,
  };
}

// --- daily budget ---

export interface DailyBudget {
  trading_date: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_cost_usd: number;
  full_pm_cycles: number;
  checkin_cycles: number;
  budget_limit_usd: number;
  budget_exhausted: boolean;
}

export function createDailyBudget(
  partial: Pick<DailyBudget, "trading_date"> & Partial<DailyBudget>,
): DailyBudget {
  return {
    total_input_tokens: 0,
    total_output_tokens: 0,
    total_cost_usd: 0.0,
    full_pm_cycles: 0,
    checkin_cycles: 0,
    budget_limit_usd: 5.0,
    budget_exhausted: false,
    ...partial,
  };
}

// --- trade record (read-only, matches trades table) ---

export interface TradeRecord {
  id: number;
  ticker: string;
  direction: string;
  entry_price: number;
  exit_price: number;
  position_size: number;
  pnl_dollars: number;
  pnl_percent: number;
  hold_duration_ms: number;
  exit_reason: string;
  entry_fill_at: string;
  exit_fill_at: string;
  entry_score_composite: number | null;
  config_version_id: number | null;
}

// --- daily performance summary ---

export interface DailyPerformance {
  trading_day: string;
  ticker: string;
  total_trades: number;
  winning_trades: number;
  avg_pnl_pct: number;
  total_pnl: number;
  win_rate: number;
  avg_hold_ms: number;
}

// --- config changelog entry ---

export interface ChangelogEntry {
  id: number;
  config_version_id: number;
  created_at: string;
  changed_by: string;
  change_category: string;
  target_timescale: string | null;
  target_tool_id: string | null;
  target_tool_type: string | null;
  target_param: string | null;
  old_value: unknown;
  new_value: unknown;
  reason: string;
}

// --- token usage tracking ---

export const MODEL_PRICING: Record<string, { input: number; output: number }> =
  {
    "claude-haiku-4-5-20251001": { input: 1.0, output: 5.0 },
    "claude-sonnet-4-5-20250514": { input: 3.0, output: 15.0 },
    "claude-sonnet-4-6": { input: 3.0, output: 15.0 },
    "claude-opus-4-6": { input: 5.0, output: 25.0 },
  };

const DEFAULT_PRICING = { input: 1.0, output: 5.0 };

export interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
  model: string;
}

export function createTokenUsage(partial?: Partial<TokenUsage>): TokenUsage {
  return {
    input_tokens: 0,
    output_tokens: 0,
    model: "claude-sonnet-4-6",
    ...partial,
  };
}

export function estimateCost(usage: TokenUsage): number {
  const pricing = MODEL_PRICING[usage.model] ?? DEFAULT_PRICING;
  const inputCost = (usage.input_tokens * pricing.input) / 1_000_000;
  const outputCost = (usage.output_tokens * pricing.output) / 1_000_000;
  return inputCost + outputCost;
}

// --- belief (CVRF) ---

export type BeliefStatus = "active" | "deprecated" | "disproven";

export interface Belief {
  id?: number;
  belief_text: string;
  confidence: number;
  evidence_count: number;
  category: string;
  status: BeliefStatus;
  source_memo_ids: number[];
  created_at?: string;
  updated_at?: string;
}

export function createBelief(
  partial: Pick<Belief, "belief_text" | "category"> & Partial<Belief>,
): Belief {
  return {
    confidence: 0.5,
    evidence_count: 1,
    status: "active",
    source_memo_ids: [],
    ...partial,
  };
}

// --- validation thresholds ---

export interface ValidationThresholds {
  max_sharpe_degradation: number;
  min_win_rate: number;
  max_drawdown_pct: number;
  min_trades: number;
}

export function createValidationThresholds(
  partial?: Partial<ValidationThresholds>,
): ValidationThresholds {
  return {
    max_sharpe_degradation: 0.5,
    min_win_rate: 0.3,
    max_drawdown_pct: 0.15,
    min_trades: 5,
    ...partial,
  };
}
