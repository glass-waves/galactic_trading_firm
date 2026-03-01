"""tests for pydantic models and token usage tracking."""

from datetime import date

from agents.models import (
    AgentMemo,
    AgentType,
    CycleType,
    DailyBudget,
    DirectionalBias,
    EvolutionCycle,
    MemoType,
    SignalQuality,
    TokenUsage,
    ValidationThresholds,
    VolatilityRegime,
)


def test_agent_memo_defaults():
    memo = AgentMemo(
        agent=AgentType.agent_5min,
        evolution_cycle_id=1,
        reasoning="test reasoning",
    )
    assert memo.memo_type == MemoType.observation
    assert memo.confidence_score is None
    assert memo.flags == {}
    assert memo.reasoning == "test reasoning"


def test_agent_memo_full():
    memo = AgentMemo(
        agent=AgentType.agent_1min,
        evolution_cycle_id=42,
        memo_type=MemoType.observation,
        confidence_score=0.85,
        volatility_regime=VolatilityRegime.high,
        directional_bias=DirectionalBias.lean_long,
        signal_quality=SignalQuality.strong,
        flags={"divergence_detected": True, "volume_anomaly": False},
        reasoning="strong momentum with divergence",
        trades_reviewed=15,
        period_win_rate=0.67,
        period_pnl=250.0,
    )
    assert memo.confidence_score == 0.85
    assert memo.volatility_regime == VolatilityRegime.high
    assert memo.flags["divergence_detected"] is True


def test_evolution_cycle_defaults():
    cycle = EvolutionCycle(trading_date=date(2024, 1, 15))
    assert cycle.cycle_type == CycleType.checkin
    assert cycle.model_used == "claude-haiku-4-5-20251001"
    assert cycle.agents_triggered == []
    assert cycle.estimated_cost_usd == 0.0


def test_daily_budget_defaults():
    budget = DailyBudget(trading_date=date(2024, 1, 15))
    assert budget.budget_limit_usd == 5.00
    assert budget.budget_exhausted is False
    assert budget.total_cost_usd == 0.0


def test_token_usage_cost_estimation():
    usage = TokenUsage(input_tokens=1_000_000, output_tokens=100_000)
    # haiku: $0.80/M input + $4.00/M output
    expected = 0.80 + 0.40
    assert abs(usage.estimated_cost_usd - expected) < 0.001


def test_token_usage_zero():
    usage = TokenUsage()
    assert usage.estimated_cost_usd == 0.0


def test_token_usage_small():
    usage = TokenUsage(input_tokens=1000, output_tokens=500)
    # 1000 * 0.80/1M + 500 * 4.00/1M = 0.0008 + 0.002 = 0.0028
    assert abs(usage.estimated_cost_usd - 0.0028) < 0.0001


def test_memo_serialization():
    memo = AgentMemo(
        agent=AgentType.agent_hourly,
        evolution_cycle_id=1,
        reasoning="test",
        confidence_score=0.5,
    )
    data = memo.model_dump()
    assert data["agent"] == "agent_hourly"
    assert data["confidence_score"] == 0.5

    # roundtrip
    restored = AgentMemo(**data)
    assert restored.agent == AgentType.agent_hourly
    assert restored.confidence_score == 0.5


def test_agent_type_enum_values():
    assert AgentType.agent_1min.value == "agent_1min"
    assert AgentType.agent_pm.value == "agent_pm"
    assert AgentType.orchestrator.value == "orchestrator"


def test_token_usage_sonnet_pricing():
    usage = TokenUsage(
        input_tokens=1_000_000,
        output_tokens=100_000,
        model="claude-sonnet-4-5-20250514",
    )
    # sonnet: $3.00/M input + $15.00/M output
    expected = 3.00 + 1.50
    assert abs(usage.estimated_cost_usd - expected) < 0.001


def test_token_usage_haiku_unchanged():
    """haiku pricing should remain the same as before."""
    usage = TokenUsage(input_tokens=1_000_000, output_tokens=100_000)
    # haiku: $0.80/M input + $4.00/M output
    expected = 0.80 + 0.40
    assert abs(usage.estimated_cost_usd - expected) < 0.001
    assert usage.model == "claude-haiku-4-5-20251001"


def test_token_usage_unknown_model_fallback():
    usage = TokenUsage(
        input_tokens=1_000_000,
        output_tokens=100_000,
        model="claude-unknown-model",
    )
    # falls back to haiku pricing
    expected = 0.80 + 0.40
    assert abs(usage.estimated_cost_usd - expected) < 0.001


def test_validation_thresholds_defaults():
    thresholds = ValidationThresholds()
    assert thresholds.max_sharpe_degradation == 0.5
    assert thresholds.min_win_rate == 0.30
    assert thresholds.max_drawdown_pct == 0.15
    assert thresholds.min_trades == 5
