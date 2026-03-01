"""pydantic models matching the postgres schema.

these models serve as the data layer between the database and the agent code.
they match the tables defined in docs/data_model.sql.
"""

from __future__ import annotations

import enum
from datetime import date, datetime
from typing import Any

from pydantic import BaseModel, Field


class AgentType(str, enum.Enum):
    agent_1min = "agent_1min"
    agent_5min = "agent_5min"
    agent_hourly = "agent_hourly"
    agent_daily = "agent_daily"
    agent_monthly = "agent_monthly"
    agent_pm = "agent_pm"
    orchestrator = "orchestrator"


class MemoType(str, enum.Enum):
    observation = "observation"
    recommendation = "recommendation"


class CycleType(str, enum.Enum):
    full_pm = "full_pm"
    checkin = "checkin"


class VolatilityRegime(str, enum.Enum):
    low = "low"
    normal = "normal"
    high = "high"
    extreme = "extreme"


class DirectionalBias(str, enum.Enum):
    strong_long = "strong_long"
    lean_long = "lean_long"
    neutral = "neutral"
    lean_short = "lean_short"
    strong_short = "strong_short"


class SignalQuality(str, enum.Enum):
    strong = "strong"
    moderate = "moderate"
    weak = "weak"
    conflicting = "conflicting"


# --- agent memo ---


class AgentMemo(BaseModel):
    """structured observation or recommendation from an agent."""

    agent: AgentType
    evolution_cycle_id: int
    memo_type: MemoType = MemoType.observation
    confidence_score: float | None = None
    volatility_regime: VolatilityRegime | None = None
    directional_bias: DirectionalBias | None = None
    signal_quality: SignalQuality | None = None
    flags: dict[str, Any] = Field(default_factory=dict)
    reasoning: str = ""
    proposed_config_version_id: int | None = None
    review_period_start: datetime | None = None
    review_period_end: datetime | None = None
    trades_reviewed: int | None = None
    period_win_rate: float | None = None
    period_sharpe: float | None = None
    period_pnl: float | None = None


# --- evolution cycle ---


class EvolutionCycle(BaseModel):
    """record of a single evolution cycle run."""

    trading_date: date
    cycle_type: CycleType = CycleType.checkin
    model_used: str = "claude-haiku-4-5-20251001"
    agents_triggered: list[AgentType] = Field(default_factory=list)
    agents_completed: list[AgentType] = Field(default_factory=list)
    configs_proposed: int = 0
    configs_promoted: int = 0
    configs_rejected: int = 0
    day_total_trades: int | None = None
    day_total_pnl: float | None = None
    day_win_rate: float | None = None
    day_sharpe: float | None = None
    input_tokens_used: int = 0
    output_tokens_used: int = 0
    estimated_cost_usd: float = 0.0


# --- daily budget ---


class DailyBudget(BaseModel):
    """daily api spend tracking for budget enforcement."""

    trading_date: date
    total_input_tokens: int = 0
    total_output_tokens: int = 0
    total_cost_usd: float = 0.0
    full_pm_cycles: int = 0
    checkin_cycles: int = 0
    budget_limit_usd: float = 5.00
    budget_exhausted: bool = False


# --- trade record (read-only, matches trades table) ---


class TradeRecord(BaseModel):
    """trade record from the trades table. agents have read-only access."""

    id: int
    ticker: str
    direction: str
    entry_price: float
    exit_price: float
    position_size: float
    pnl_dollars: float
    pnl_percent: float
    hold_duration_ms: int
    exit_reason: str
    entry_fill_at: datetime
    exit_fill_at: datetime
    entry_score_composite: float | None = None
    config_version_id: int | None = None


# --- daily performance summary ---


class DailyPerformance(BaseModel):
    """aggregated daily performance stats."""

    trading_day: date
    ticker: str
    total_trades: int
    winning_trades: int
    avg_pnl_pct: float
    total_pnl: float
    win_rate: float
    avg_hold_ms: float


# --- config changelog entry ---


class ChangelogEntry(BaseModel):
    """a single config change from the changelog."""

    id: int
    config_version_id: int
    created_at: datetime
    changed_by: str
    change_category: str
    target_timescale: str | None = None
    target_tool_id: str | None = None
    target_tool_type: str | None = None
    target_param: str | None = None
    old_value: Any | None = None
    new_value: Any
    reason: str


# --- token usage tracking ---


MODEL_PRICING: dict[str, dict[str, float]] = {
    "claude-haiku-4-5-20251001": {"input": 0.80, "output": 4.00},
    "claude-sonnet-4-5-20250514": {"input": 3.00, "output": 15.00},
}

# fallback to haiku pricing for unknown models
_DEFAULT_PRICING = {"input": 0.80, "output": 4.00}


class TokenUsage(BaseModel):
    """token usage from a single api call."""

    input_tokens: int = 0
    output_tokens: int = 0
    model: str = "claude-haiku-4-5-20251001"

    @property
    def estimated_cost_usd(self) -> float:
        """estimate cost based on model pricing."""
        pricing = MODEL_PRICING.get(self.model, _DEFAULT_PRICING)
        input_cost = self.input_tokens * pricing["input"] / 1_000_000
        output_cost = self.output_tokens * pricing["output"] / 1_000_000
        return input_cost + output_cost


# --- validation thresholds ---


class ValidationThresholds(BaseModel):
    """thresholds for backtest validation of proposed configs."""

    max_sharpe_degradation: float = 0.5
    min_win_rate: float = 0.30
    max_drawdown_pct: float = 0.15
    min_trades: int = 5
