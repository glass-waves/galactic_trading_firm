// shared domain types — mirrors postgres schema and rust types

export interface Trade {
  id: number;
  ticker: string;
  direction: "long" | "short";
  entry_price: number;
  exit_price: number;
  position_size: number;
  pnl_dollars: number;
  pnl_percent: number;
  hold_duration_ms: number;
  exit_reason: string;
  entry_signal_at: string;
  entry_fill_at: string;
  exit_signal_at: string;
  exit_fill_at: string;
  entry_score_1min: number | null;
  entry_score_5min: number | null;
  entry_score_hourly: number | null;
  entry_score_composite: number | null;
  exit_score_1min: number | null;
  exit_score_5min: number | null;
  exit_score_hourly: number | null;
  exit_score_composite: number | null;
  high_water_mark: number | null;
  low_water_mark: number | null;
  config_version_id: number | null;
  is_paper: boolean;
}

export interface ConfigVersion {
  id: number;
  status: string;
  created_at: string;
  promoted_at: string | null;
  created_by: string | null;
  parent_version_id: number | null;
  mutation_reason: string | null;
  config_blob: Record<string, unknown>;
  backtest_sharpe: number | null;
  backtest_win_rate: number | null;
  backtest_total_trades: number | null;
}

export interface DailyPerformance {
  trading_day: string;
  ticker: string;
  total_trades: number;
  winning_trades: number;
  total_pnl: number;
  win_rate: number;
  avg_pnl_pct: number;
  avg_hold_ms: number;
}

export interface AgentMemo {
  id: number;
  agent: string;
  evolution_cycle_id: number | null;
  memo_type: string;
  confidence_score: number | null;
  volatility_regime: string | null;
  directional_bias: string | null;
  signal_quality: string | null;
  reasoning: string | null;
  suggestions: unknown;
  created_at: string;
}

export interface Belief {
  id: number;
  belief_text: string;
  confidence: number;
  evidence_count: number;
  category: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface EvolutionCycle {
  id: number;
  trading_date: string;
  cycle_type: string;
  model_used: string;
  started_at: string;
  completed_at: string | null;
  input_tokens_used: number;
  output_tokens_used: number;
  estimated_cost_usd: number;
  configs_proposed: number;
  configs_promoted: number;
  configs_rejected: number;
}

export interface DailyBudget {
  trading_date: string;
  total_cost_usd: number;
  budget_limit_usd: number;
  budget_exhausted: boolean;
  full_pm_cycles: number;
  checkin_cycles: number;
}

export interface ConfigChangelog {
  id: number;
  config_version_id: number;
  created_at: string;
  changed_by: string;
  change_category: string;
  target_tool_id: string | null;
  target_param: string | null;
  old_value: unknown;
  new_value: unknown;
  reason: string | null;
}
