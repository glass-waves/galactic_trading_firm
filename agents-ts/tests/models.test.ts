import { describe, it, expect } from "vitest";
import {
  type AgentType,
  type Belief,
  type BeliefStatus,
  type Suggestion,
  createAgentMemo,
  createBelief,
  createEvolutionCycle,
  createDailyBudget,
  createTokenUsage,
  createValidationThresholds,
  estimateCost,
} from "../src/models.js";

describe("AgentMemo", () => {
  it("should use defaults", () => {
    const memo = createAgentMemo({
      agent: "agent_analysis",
      evolution_cycle_id: 1,
      reasoning: "test reasoning",
    });
    expect(memo.memo_type).toBe("observation");
    expect(memo.confidence_score).toBeNull();
    expect(memo.flags).toEqual({});
    expect(memo.reasoning).toBe("test reasoning");
  });

  it("should accept full fields", () => {
    const memo = createAgentMemo({
      agent: "agent_analysis",
      evolution_cycle_id: 42,
      memo_type: "observation",
      confidence_score: 0.85,
      volatility_regime: "high",
      directional_bias: "lean_long",
      signal_quality: "strong",
      flags: { divergence_detected: true, volume_anomaly: false },
      reasoning: "strong momentum with divergence",
      trades_reviewed: 15,
      period_win_rate: 0.67,
      period_pnl: 250.0,
    });
    expect(memo.confidence_score).toBe(0.85);
    expect(memo.volatility_regime).toBe("high");
    expect(memo.flags.divergence_detected).toBe(true);
  });
});

describe("EvolutionCycle", () => {
  it("should use defaults", () => {
    const cycle = createEvolutionCycle({ trading_date: "2024-01-15" });
    expect(cycle.cycle_type).toBe("checkin");
    expect(cycle.model_used).toBe("claude-sonnet-4-6");
    expect(cycle.agents_triggered).toEqual([]);
    expect(cycle.estimated_cost_usd).toBe(0.0);
  });
});

describe("DailyBudget", () => {
  it("should use defaults", () => {
    const budget = createDailyBudget({ trading_date: "2024-01-15" });
    expect(budget.budget_limit_usd).toBe(5.0);
    expect(budget.budget_exhausted).toBe(false);
    expect(budget.total_cost_usd).toBe(0.0);
  });
});

describe("TokenUsage", () => {
  it("should estimate default (sonnet 4.6) cost", () => {
    const usage = createTokenUsage({
      input_tokens: 1_000_000,
      output_tokens: 100_000,
    });
    // sonnet 4.6: $3.00/M input + $15.00/M output
    const expected = 3.0 + 1.5;
    expect(Math.abs(estimateCost(usage) - expected)).toBeLessThan(0.001);
  });

  it("should return zero for zero tokens", () => {
    const usage = createTokenUsage();
    expect(estimateCost(usage)).toBe(0.0);
  });

  it("should estimate small usage", () => {
    const usage = createTokenUsage({
      input_tokens: 1000,
      output_tokens: 500,
    });
    // default sonnet 4.6: 1000 * 3.00/1M + 500 * 15.00/1M = 0.003 + 0.0075 = 0.0105
    expect(Math.abs(estimateCost(usage) - 0.0105)).toBeLessThan(0.0001);
  });

  it("should estimate sonnet pricing", () => {
    const usage = createTokenUsage({
      input_tokens: 1_000_000,
      output_tokens: 100_000,
      model: "claude-sonnet-4-5-20250514",
    });
    // sonnet: $3.00/M input + $15.00/M output
    const expected = 3.0 + 1.5;
    expect(Math.abs(estimateCost(usage) - expected)).toBeLessThan(0.001);
  });

  it("should estimate haiku pricing when explicitly specified", () => {
    const usage = createTokenUsage({
      input_tokens: 1_000_000,
      output_tokens: 100_000,
      model: "claude-haiku-4-5-20251001",
    });
    // haiku 4.5: $1.00/M input + $5.00/M output
    const expected = 1.0 + 0.5;
    expect(Math.abs(estimateCost(usage) - expected)).toBeLessThan(0.001);
    expect(usage.model).toBe("claude-haiku-4-5-20251001");
  });

  it("should fall back to default pricing for unknown model", () => {
    const usage = createTokenUsage({
      input_tokens: 1_000_000,
      output_tokens: 100_000,
      model: "claude-unknown-model",
    });
    // falls back to default (haiku 4.5) pricing: $1.00/M + $5.00/M
    const expected = 1.0 + 0.5;
    expect(Math.abs(estimateCost(usage) - expected)).toBeLessThan(0.001);
  });
});

describe("memo serialization", () => {
  it("should roundtrip via JSON", () => {
    const memo = createAgentMemo({
      agent: "agent_analysis",
      evolution_cycle_id: 1,
      reasoning: "test",
      confidence_score: 0.5,
    });
    const data = JSON.parse(JSON.stringify(memo));
    expect(data.agent).toBe("agent_analysis");
    expect(data.confidence_score).toBe(0.5);

    const restored = createAgentMemo(data);
    expect(restored.agent).toBe("agent_analysis");
    expect(restored.confidence_score).toBe(0.5);
  });
});

describe("AgentType values", () => {
  it("should have correct string values", () => {
    const types: AgentType[] = [
      "agent_analysis",
      "agent_pm",
      "orchestrator",
    ];
    expect(types[0]).toBe("agent_analysis");
    expect(types[1]).toBe("agent_pm");
    expect(types[2]).toBe("orchestrator");
  });
});

describe("Suggestion", () => {
  it("should have all required fields", () => {
    const s: Suggestion = {
      target_tool_id: "rsi_7_1min",
      param: "period",
      current_value: 7,
      proposed_value: 10,
      confidence: 0.8,
      evidence_summary: "lower RSI period causing whipsaw",
    };
    expect(s.target_tool_id).toBe("rsi_7_1min");
    expect(s.param).toBe("period");
    expect(s.confidence).toBe(0.8);
  });

  it("should allow nullable param", () => {
    const s: Suggestion = {
      target_tool_id: "new_indicator",
      param: null,
      current_value: null,
      proposed_value: { indicator_type: "mfi", weight: 0.15 },
      confidence: 0.6,
      evidence_summary: "adding volume signal",
    };
    expect(s.param).toBeNull();
    expect(s.current_value).toBeNull();
  });

  it("memo defaults to empty suggestions", () => {
    const memo = createAgentMemo({
      agent: "agent_analysis",
      evolution_cycle_id: 1,
    });
    expect(memo.suggestions).toEqual([]);
  });

  it("memo with suggestions populated", () => {
    const suggestions: Suggestion[] = [
      {
        target_tool_id: "rsi_7_1min",
        param: "period",
        current_value: 7,
        proposed_value: 10,
        confidence: 0.8,
        evidence_summary: "test evidence",
      },
    ];
    const memo = createAgentMemo({
      agent: "agent_analysis",
      evolution_cycle_id: 1,
      suggestions,
    });
    expect(memo.suggestions).toHaveLength(1);
    expect(memo.suggestions[0].target_tool_id).toBe("rsi_7_1min");
  });
});

describe("Belief", () => {
  it("should have all required fields", () => {
    const b: Belief = {
      belief_text: "trailing stops should be wider in high volatility",
      confidence: 0.8,
      evidence_count: 3,
      category: "risk_management",
      status: "active",
      source_memo_ids: [1, 2, 3],
    };
    expect(b.belief_text).toContain("trailing stops");
    expect(b.confidence).toBe(0.8);
    expect(b.category).toBe("risk_management");
  });

  it("should create with defaults", () => {
    const b = createBelief({
      belief_text: "test belief",
      category: "indicator_tuning",
    });
    expect(b.confidence).toBe(0.5);
    expect(b.evidence_count).toBe(1);
    expect(b.status).toBe("active");
    expect(b.source_memo_ids).toEqual([]);
  });

  it("should support status values", () => {
    const statuses: BeliefStatus[] = ["active", "deprecated", "disproven"];
    expect(statuses).toHaveLength(3);
    for (const s of statuses) {
      const b = createBelief({
        belief_text: "test",
        category: "test",
        status: s,
      });
      expect(b.status).toBe(s);
    }
  });

  it("confidence range should be 0 to 1", () => {
    const b = createBelief({
      belief_text: "test",
      category: "test",
      confidence: 0.75,
    });
    expect(b.confidence).toBeGreaterThanOrEqual(0);
    expect(b.confidence).toBeLessThanOrEqual(1);
  });
});

describe("ValidationThresholds", () => {
  it("should use defaults", () => {
    const t = createValidationThresholds();
    expect(t.max_sharpe_degradation).toBe(0.5);
    expect(t.min_win_rate).toBe(0.3);
    expect(t.max_drawdown_pct).toBe(0.15);
    expect(t.min_trades).toBe(5);
  });
});
